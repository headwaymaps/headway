#[macro_use]
extern crate log;

mod error {
    pub type Error = Box<dyn std::error::Error>;
    pub type Result<T> = std::result::Result<T, Error>;
}

use error::Result;
use resource::Resource;
use std::env;
use std::path::PathBuf;

fn main() {
    env_logger::init();

    match env::var("TMPDIR") {
        Ok(val) => println!("TMPDIR was set: {val}"),
        Err(_e) => {
            let default = "./tmp";
            println!("TMPDIR was not set. Assuming default: {default:?}");
            env::set_var("TMPDIR", default);
            std::fs::create_dir_all(default).unwrap();
        }
    }

    let mut args = env::args();
    let bin_name = args.next().unwrap();

    let example_usage = format!("{bin_name} v1.36 ../../data/");
    let Some(version) = args.next() else {
        panic!("must specify _version_ arg\nusage: {example_usage}");
    };

    let Some(output_root) = args.next() else {
        panic!("must specify _output_root_ arg\nusage: {example_usage}");
    };

    let output_root = PathBuf::from(output_root);
    if !output_root.is_dir() {
        panic!("directory at output_root does not exist: {output_root:?}")
    }

    // Download
    info!("starting download of {version}");
    let dist = Distribution::new(version.to_string(), &output_root);
    dist.download_and_build().unwrap();
    info!("done with {version}");
}

use distribution::Distribution;

mod resource {
    use std::path::{Path, PathBuf};

    #[derive(Debug)]
    pub struct Resource {
        name: String,
        local_path: PathBuf,
    }

    impl Resource {
        pub fn new(name: String, local_path: PathBuf) -> Self {
            Self { name, local_path }
        }

        pub fn local_path(&self) -> &Path {
            &self.local_path
        }

        pub fn name(&self) -> &str {
            &self.name
        }
    }
}

mod distribution {
    use crate::resource::Resource;
    use crate::Result;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    pub struct Distribution {
        version: String,
        resources: Resources,
    }
    struct ResourceBuilder {
        download_dir: PathBuf,
        generated_dir: PathBuf,
    }
    impl ResourceBuilder {
        fn new(version: String, data_root: PathBuf) -> Self {
            Self {
                download_dir: data_root.join("download").join(&version),
                generated_dir: data_root.join("generated").join(&version),
            }
        }
        fn downloadable(&self, name: String) -> Resource {
            let path = self.download_dir.join(&name);
            Resource::new(name, path)
        }
        fn generated(&self, name: String) -> Resource {
            let path = self.generated_dir.join(&name);
            Resource::new(name, path)
        }
    }

    struct Resources {
        resource_builder: ResourceBuilder,
        planet_pbf: Resource,
        buildings_pbf: Resource,
        admin_osc: Resource,
        merged_planet_and_ml_buildings_pbf: Resource,
        final_planet_pbf: Resource,
    }

    impl Resources {
        pub fn new(version: &str, data_root: &Path) -> Self {
            let resource_builder =
                ResourceBuilder::new(version.to_string(), data_root.to_path_buf());
            Self {
                planet_pbf: resource_builder.downloadable(format!("planet-{version}.osm.pbf")),
                admin_osc: resource_builder.downloadable(format!("admin-{version}.osc.gz")),
                buildings_pbf: resource_builder
                    .downloadable(format!("ml-buildings-{version}.osm.pbf")),
                merged_planet_and_ml_buildings_pbf: resource_builder
                    .generated(format!("merged-planet-and-ml-buildings-{version}.osm.pbf")),
                final_planet_pbf: resource_builder
                    .generated(format!("final-planet-{version}.osm.pbf")),
                resource_builder,
            }
        }

        pub fn download_dir(&self) -> &Path {
            &self.resource_builder.download_dir
        }

        pub fn ensure_download_dir(&self) -> Result<()> {
            let dir = self.download_dir();
            std::fs::create_dir_all(dir)?;
            Ok(())
        }

        pub fn generated_dir(&self) -> &Path {
            &self.resource_builder.generated_dir
        }

        pub fn ensure_generated_dir(&self) -> Result<()> {
            let dir = self.generated_dir();
            std::fs::create_dir_all(dir)?;
            Ok(())
        }
    }

    impl Distribution {
        pub fn new(version: String, output_root: &Path) -> Self {
            Self {
                resources: Resources::new(&version, output_root),
                version,
            }
        }

        fn download(&self, resource: &Resource) -> Result<()> {
            // https://daylight-map-distribution.s3.us-west-1.amazonaws.com/release/v1.32/admin-v1.32.osc.gz
            let url =
                format!("https://daylight-map-distribution.s3.us-west-1.amazonaws.com/release/{version}/{resource_name}", version=self.version, resource_name=resource.name());

            let mut cmd = Command::new("aria2c");
            cmd.args(["-x", "16"])
                .args(["-s", "16"])
                .arg(&format!(
                    "-d {download_dir}",
                    download_dir = self
                        .resources
                        .download_dir()
                        .to_str()
                        .expect("valid UTF-8 string")
                ))
                .arg(url);

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(err) => {
                    if let std::io::ErrorKind::NotFound = err.kind() {
                        eprintln!("aria2c is missing. Install aria2c and try again: `apt install aria2c`");
                    } else {
                        eprintln!("err running aria2c: {err:?}");
                    }

                    return Err(err.into());
                }
            };

            let status = child.wait()?;
            if !status.success() {
                return Err(format!("download exited with status: {status:?}").into());
            }
            Ok(())
        }

        fn download_if_missing(&self, resource: &Resource) -> Result<()> {
            self.resources.ensure_download_dir()?;

            let existing_download = resource.local_path();
            let in_progress_download = {
                let mut os_string = OsString::from(existing_download);
                // `Path` has no API to add an additional extension without clobbering the existing extension
                os_string.push(".aria2");
                PathBuf::from(os_string)
            };
            if existing_download.exists() && !in_progress_download.exists() {
                info!("{resource:?}: Already downloaded at {existing_download:?}.");
                Ok(())
            } else {
                if existing_download.exists() {
                    info!("{resource:?}: Resuming previously started download from {in_progress_download:?}.");
                } else {
                    info!("{resource:?}: Starting download.");
                }
                self.download(resource)
            }
        }

        pub fn download_and_build(&self) -> Result<()> {
            self.download_if_missing(&self.resources.admin_osc)?;
            self.download_if_missing(&self.resources.planet_pbf)?;
            self.download_if_missing(&self.resources.buildings_pbf)?;

            crate::osmio::verify_installation()?;

            eprintln!("adding buildings");
            self.add_buildings()?;

            eprintln!("adding admin");
            self.add_admin()?;

            Ok(())
        }

        pub fn add_buildings(&self) -> Result<()> {
            self.resources.ensure_generated_dir()?;
            crate::osmio::Merge::output(&self.resources.merged_planet_and_ml_buildings_pbf)
                .input(&self.resources.planet_pbf)
                .input(&self.resources.buildings_pbf)
                .run()
        }

        pub fn add_admin(&self) -> Result<()> {
            self.resources.ensure_generated_dir()?;
            crate::osmio::ApplyChanges::output(&self.resources.final_planet_pbf)
                .input(&self.resources.merged_planet_and_ml_buildings_pbf)
                .input(&self.resources.admin_osc)
                .run()
        }
    }
}

mod osmio {
    use crate::Resource;
    use crate::Result;
    use std::env::temp_dir;
    use std::process::Command;

    pub fn verify_installation() -> Result<()> {
        let mut cmd = Command::new("osmium");
        cmd.arg("version");
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(err) => {
                if let std::io::ErrorKind::NotFound = err.kind() {
                    eprintln!("osmium is missing. Install osmium and try again: `apt install osmium-tool`");
                }
                return Err(err.into());
            }
        };

        let status = child.wait()?;
        if !status.success() {
            return Err(format!("`osmium version` exited with status: {status:?}").into());
        }

        Ok(())
    }

    #[derive(Debug)]
    pub struct Merge<'a> {
        output: &'a Resource,
        inputs: Vec<&'a Resource>,
    }

    impl<'a> Merge<'a> {
        pub fn output(output: &'a Resource) -> Self {
            Self {
                output,
                inputs: vec![],
            }
        }

        pub fn input(mut self, input: &'a Resource) -> Self {
            self.inputs.push(input);
            self
        }

        pub fn run(self) -> Result<()> {
            if self.output.local_path().exists() {
                info!("output already exists at {:?}", self.output);
                return Ok(());
            }

            let wip_output = temp_dir().join(self.output.name());

            // TODO: Merge not sort
            // osmium sort --strategy=multipass --progress -v --output merged-sorted-oscs-${VERSION}.osm.pbf "admin-${VERSION}.osc.bz2" "ms-ml-buildings-${VERSION}.osc.bz2" --overwrite
            assert!(self.inputs.len() > 1);
            let mut cmd = Command::new("osmium");
            cmd.arg("merge")
                .arg("--progress")
                .arg("-v")
                .arg(format!("--output={}", wip_output.to_str().unwrap()));

            for input in self.inputs {
                cmd.arg(input.local_path());
            }

            debug!("Running merge cmd: {cmd:?}");
            let mut child = cmd.spawn()?;
            let status = child.wait()?;
            if !status.success() {
                return Err(format!("merged exited with status: {status:?}").into());
            }

            debug!("Merge succeeded.");
            std::fs::rename(wip_output, self.output.local_path())?;

            Ok(())
        }
    }

    /// osmium apply-changes planet.osm.pbf admins.osc.gz -o everything.osm.pbf
    pub struct ApplyChanges<'a> {
        output: &'a Resource,
        inputs: Vec<&'a Resource>,
    }

    impl<'a> ApplyChanges<'a> {
        pub fn output(output: &'a Resource) -> Self {
            Self {
                output,
                inputs: vec![],
            }
        }

        pub fn input(mut self, input: &'a Resource) -> Self {
            self.inputs.push(input);
            self
        }

        pub fn run(self) -> Result<()> {
            if self.output.local_path().exists() {
                info!("output already exists at {:?}", self.output);
                return Ok(());
            }

            let wip_output = temp_dir().join(self.output.name());

            // TODO: Merge not sort
            // osmium sort --strategy=multipass --progress -v --output merged-sorted-oscs-${VERSION}.osm.pbf "admin-${VERSION}.osc.bz2" "ms-ml-buildings-${VERSION}.osc.bz2" --overwrite
            assert!(self.inputs.len() > 1);
            let mut cmd = Command::new("osmium");
            cmd.arg("apply-changes")
                .arg("--progress")
                .arg("-v")
                .arg(format!("--output={}", wip_output.to_str().unwrap()));

            for input in self.inputs {
                cmd.arg(input.local_path());
            }

            debug!("Running merge cmd: {cmd:?}");
            let mut child = cmd.spawn()?;
            let status = child.wait()?;
            if !status.success() {
                return Err(format!("merged exited with status: {status:?}").into());
            }

            debug!("Merge succeeded.");
            std::fs::rename(wip_output, self.output.local_path())?;

            Ok(())
        }
    }
}
