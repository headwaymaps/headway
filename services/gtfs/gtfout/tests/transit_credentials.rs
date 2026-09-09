use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "transit-credentials-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(path.join("build/transit/metro")).unwrap();
        fs::write(path.join("build/transit/metro/zone.json"), r#"{
          "version": 1,
          "bounds": {"min_lon": 0, "min_lat": 0, "max_lon": 1, "max_lat": 1},
          "feeds": [{"feed_onestop_id": "f-metro", "provider": "Metro", "url": "https://example.com/gtfs.zip",
            "authorization": {"type": "query_param", "param_name": "key", "info_url": "https://example.com/keys"}}]
        }"#).unwrap();
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_transit-credentials"))
            .current_dir(&self.0)
            .arg("build")
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn writes_only_the_credentials_the_zone_needs() {
    let fixture = Fixture::new();
    // A key with characters that have to survive a round trip through JSON.
    fs::write(
        fixture.0.join("gtfs-secrets.json"),
        r#"[{"feed_id": "f-metro", "key": "to\"ken=a#b"},
            {"feed_id": "f-other", "key": "unused"}]"#,
    )
    .unwrap();
    let output = fixture.run(&[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let path = fixture.0.join("build/transit/metro/gtfs-secrets.json");
    let contents = fs::read_to_string(&path).unwrap();
    // The zone's file is the same format, so gtfout reads it back unchanged.
    let reloaded: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(reloaded[0]["feed_id"], "f-metro");
    assert_eq!(reloaded[0]["key"], "to\"ken=a#b");
    assert_eq!(reloaded.as_array().unwrap().len(), 1);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ken=a"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn missing_credentials_report_the_feed_and_signup_url() {
    let fixture = Fixture::new();
    let output = fixture.run(&[]);
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#""feed_id": "f-metro""#), "{stdout}");
    assert!(stdout.contains("https://example.com/keys"), "{stdout}");
    // Still written, so a partly-filled deployment says which feed is short.
    assert_eq!(
        fs::read_to_string(fixture.0.join("build/transit/metro/gtfs-secrets.json")).unwrap(),
        "[]\n"
    );
}

#[test]
fn verification_does_not_write_files_and_rejects_unmatched_credentials() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("gtfs-secrets.json"),
        r#"[{"feed_id": "f-metro", "key": "token"}]"#,
    )
    .unwrap();
    fs::create_dir_all(fixture.0.join("atlas/feeds")).unwrap();
    let output = fixture.run(&["--verify", "--atlas-path", "atlas"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("no authenticated Atlas endpoint for f-metro"));
    assert!(!fixture
        .0
        .join("build/transit/metro/gtfs-secrets.json")
        .exists());
}

#[test]
fn invalid_zones_fail_before_any_credentials_are_written() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("build/transit/other")).unwrap();
    fs::write(fixture.0.join("build/transit/other/zone.json"), "{}").unwrap();
    assert!(!fixture.run(&[]).status.success());
    assert!(!fixture
        .0
        .join("build/transit/metro/gtfs-secrets.json")
        .exists());
}

#[test]
fn verification_with_missing_tokens_does_not_overwrite_existing_credentials() {
    let fixture = Fixture::new();
    let path = fixture.0.join("build/transit/metro/gtfs-secrets.json");
    fs::write(&path, "existing").unwrap();
    assert!(!fixture.run(&["--verify"]).status.success());
    assert_eq!(fs::read_to_string(path).unwrap(), "existing");
}
