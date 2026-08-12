//! Getting a transitland-atlas clone onto disk.
//!
//! Cloning it by hand stays a fine way to work, so `--atlas-path` alone is
//! read-only and never touches the network. `--download` opts into managing the
//! clone; it exists to stop a mistyped path silently becoming a fresh clone
//! rather than an error, since the tool goes on to download GB of feeds anyway.

use crate::Result;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Upstream, unless [`REPO_ENV_VAR`] says otherwise.
pub const DEFAULT_REPO: &str = "https://github.com/transitland/transitland-atlas.git";

/// The ref to track, unless [`REF_ENV_VAR`] says otherwise.
pub const DEFAULT_REF: &str = "main";

/// Read by the dagger build too, so a pinned atlas means the same thing whether
/// you go through dagger or run the CLI directly.
pub const REPO_ENV_VAR: &str = "HEADWAY_TRANSITLAND_ATLAS_URL";
pub const REF_ENV_VAR: &str = "HEADWAY_TRANSITLAND_ATLAS_REF";

pub fn default_repo() -> String {
    std::env::var(REPO_ENV_VAR).unwrap_or_else(|_| DEFAULT_REPO.to_owned())
}

pub fn default_ref() -> String {
    std::env::var(REF_ENV_VAR).unwrap_or_else(|_| DEFAULT_REF.to_owned())
}

/// Where the atlas is, and whether we're allowed to fetch it.
#[derive(Debug, Clone)]
pub struct AtlasSource {
    pub path: PathBuf,
    /// Clone if absent, refresh if present. Without this the path is read-only.
    pub download: bool,
    pub repo: String,
    pub git_ref: String,
}

impl AtlasSource {
    /// Makes sure there's a usable clone at `path`, and returns it.
    ///
    /// Deliberately not a validity check - whether the directory really holds an
    /// atlas is [`crate::dmfr::load_catalog`]'s call, and its error is better.
    pub fn ensure(&self) -> Result<&Path> {
        // plan() doesn't take the path, so name it here or the errors are
        // untraceable.
        let plan = plan(
            state_of(&self.path, self.download)?,
            self.download,
            &self.repo,
        )
        .map_err(|e| format!("{}: {e}", self.path.display()))?;

        match plan {
            Plan::Use => {}
            Plan::Clone => self.clone_repo()?,
            Plan::Refresh => self.refresh()?,
        }
        Ok(&self.path)
    }

    /// Shallow: nothing reads history, only feeds/ as it stands at `git_ref`.
    fn clone_repo(&self) -> Result<()> {
        eprintln!(
            "cloning {} ({}) into {}",
            self.repo,
            self.git_ref,
            self.path.display()
        );
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        git(
            None,
            &[
                "clone",
                "--depth",
                "1",
                "--branch",
                &self.git_ref,
                &self.repo,
                &self.path.to_string_lossy(),
            ],
        )
    }

    /// Fetches and hard-resets, so `--download` is idempotent. A stale atlas is
    /// the failure most worth avoiding: a feed cataloged since the clone never
    /// reaches the index, and looks exactly like an agency that doesn't exist.
    fn refresh(&self) -> Result<()> {
        eprintln!(
            "refreshing {} from {} ({})",
            self.path.display(),
            self.repo,
            self.git_ref
        );
        git(
            Some(&self.path),
            &["fetch", "--depth", "1", &self.repo, &self.git_ref],
        )?;
        git(Some(&self.path), &["reset", "--hard", "FETCH_HEAD"])?;
        // A shallow fetch leaves the previous tip dangling, so without this the
        // directory grows by a checkout per refresh.
        git(Some(&self.path), &["clean", "-fd"])
    }
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<()> {
    let mut command = Command::new("git");
    if let Some(dir) = dir {
        command.arg("-C").arg(dir);
    }
    command.args(args);

    let status = command
        .status()
        .map_err(|e| format!("running git {}: {e}", args.join(" ")))?;

    if !status.success() {
        return Err(format!("git {} failed with {status}", args.join(" ")).into());
    }
    Ok(())
}

/// What we found at the path.
#[derive(Debug, PartialEq, Eq)]
enum State {
    /// Absent, or empty - what a failed clone leaves behind, and what
    /// `git clone` is happy to write into.
    MissingOrEmpty,
    /// A git checkout, and whether `origin` is the repo we were asked for.
    GitClone { remote: Option<String> },
    /// Has content but isn't a git checkout.
    OtherContent,
}

#[derive(Debug, PartialEq, Eq)]
enum Plan {
    Use,
    Clone,
    Refresh,
}

/// Inspects the path. `need_remote` is false on the read-only path, and not
/// just as an optimization: the dagger build mounts an atlas that has a `.git`
/// into a container with no `git`, so probing unconditionally broke the one
/// caller that never asked to download anything.
fn state_of(path: &Path, need_remote: bool) -> Result<State> {
    if !path.exists() {
        return Ok(State::MissingOrEmpty);
    }
    if !path.is_dir() {
        return Err(format!("{} is not a directory", path.display()).into());
    }
    if std::fs::read_dir(path)?.next().is_none() {
        return Ok(State::MissingOrEmpty);
    }
    if !path.join(".git").exists() {
        return Ok(State::OtherContent);
    }
    if !need_remote {
        return Ok(State::GitClone { remote: None });
    }

    // Unknown rather than fatal when git won't run: the clone or fetch that
    // follows fails with a better message than we could produce here.
    let remote = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());

    Ok(State::GitClone { remote })
}

/// Decides what to do. Split from the doing so the awkward combinations are
/// testable without a network or a git binary.
fn plan(state: State, download: bool, repo: &str) -> Result<Plan> {
    match (state, download) {
        // Read-only: whatever is there is what the caller meant, and
        // load_catalog rejects it if it isn't an atlas.
        (State::GitClone { .. } | State::OtherContent, false) => Ok(Plan::Use),

        (State::MissingOrEmpty, false) => Err(format!(
            "no transitland-atlas clone at that path - clone one there, or pass --download to fetch {repo}"
        )
        .into()),

        (State::MissingOrEmpty, true) => Ok(Plan::Clone),

        (State::GitClone { remote }, true) => {
            match remote {
                Some(remote) if !same_repo(&remote, repo) => Err(format!(
                    "refusing to --download over an existing clone of {remote}, which is not {repo} - remove it, or point --atlas-path somewhere else"
                )
                .into()),
                // No origin is odd but harmless - we fetch by URL, not by name.
                _ => Ok(Plan::Refresh),
            }
        }

        (State::OtherContent, true) => Err(
            "refusing to --download over a directory that already has content and is not a git clone - remove it, or point --atlas-path somewhere else"
                .to_owned()
                .into(),
        ),
    }
}

/// Whether two remote URLs name the same repository.
///
/// Normalizes only what differs for the same URL written two ways: a trailing
/// slash or `.git`. Not clever about ssh-vs-https, because getting that wrong
/// refuses a legitimate refresh and the fix is one flag away.
fn same_repo(a: &str, b: &str) -> bool {
    fn normalize(url: &str) -> &str {
        url.trim().trim_end_matches('/').trim_end_matches(".git")
    }
    normalize(a) == normalize(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPSTREAM: &str = "https://github.com/transitland/transitland-atlas.git";

    fn git_clone(remote: &str) -> State {
        State::GitClone {
            remote: Some(remote.to_owned()),
        }
    }

    #[test]
    fn without_download_an_existing_directory_is_used_as_is() {
        assert_eq!(
            plan(git_clone(UPSTREAM), false, UPSTREAM).unwrap(),
            Plan::Use
        );
        // Even a clone of something else: no --download means the caller is
        // responsible for what's there, and a fork is a legitimate thing to
        // point at.
        assert_eq!(
            plan(git_clone("https://example.com/fork"), false, UPSTREAM).unwrap(),
            Plan::Use
        );
        // Not a git checkout at all - an exported tarball, say.
        assert_eq!(
            plan(State::OtherContent, false, UPSTREAM).unwrap(),
            Plan::Use
        );
    }

    #[test]
    fn the_read_only_path_never_looks_up_the_remote() {
        // The dagger build mounts a checkout that has a .git into a container
        // with no git binary, and only ever reads it. Probing the remote there
        // failed the whole run with "running git remote get-url: No such file
        // or directory".
        let temp = std::env::temp_dir().join(format!("gtfout-atlas-{}", std::process::id()));
        std::fs::create_dir_all(temp.join(".git")).unwrap();
        std::fs::create_dir_all(temp.join("feeds")).unwrap();

        let state = state_of(&temp, false).unwrap();
        assert_eq!(state, State::GitClone { remote: None });
        assert_eq!(plan(state, false, UPSTREAM).unwrap(), Plan::Use);

        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn without_download_a_missing_path_is_an_error_naming_the_flag() {
        let err = plan(State::MissingOrEmpty, false, UPSTREAM)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--download"), "{err}");
    }

    #[test]
    fn download_clones_when_there_is_nothing_there() {
        // An empty directory counts as nothing: it's what a failed clone leaves
        // behind, and git is happy to clone into it.
        assert_eq!(
            plan(State::MissingOrEmpty, true, UPSTREAM).unwrap(),
            Plan::Clone
        );
    }

    #[test]
    fn download_refreshes_an_existing_clone_of_the_same_repo() {
        // The point of the flag: running it twice is idempotent rather than an
        // error, so the atlas can't quietly go stale.
        assert_eq!(
            plan(git_clone(UPSTREAM), true, UPSTREAM).unwrap(),
            Plan::Refresh
        );
    }

    #[test]
    fn download_will_not_clobber_a_clone_of_a_different_repo() {
        let err = plan(git_clone("https://example.com/other"), true, UPSTREAM)
            .unwrap_err()
            .to_string();
        assert!(err.contains("example.com/other"), "{err}");
    }

    #[test]
    fn download_will_not_clobber_an_unrelated_directory() {
        assert!(plan(State::OtherContent, true, UPSTREAM).is_err());
    }

    #[test]
    fn a_url_written_two_ways_is_still_the_same_repo() {
        assert!(same_repo(UPSTREAM, UPSTREAM));
        assert!(same_repo(
            "https://github.com/transitland/transitland-atlas",
            UPSTREAM
        ));
        assert!(same_repo(
            "https://github.com/transitland/transitland-atlas/",
            UPSTREAM
        ));
        assert_eq!(
            plan(
                git_clone("https://github.com/transitland/transitland-atlas"),
                true,
                UPSTREAM
            )
            .unwrap(),
            Plan::Refresh
        );
    }

    #[test]
    fn different_repos_are_not_confused_by_normalization() {
        assert!(!same_repo("https://example.com/a", "https://example.com/b"));
        assert!(!same_repo(
            "https://github.com/someone/transitland-atlas",
            UPSTREAM
        ));
    }

    #[test]
    fn a_checkout_with_no_origin_still_refreshes() {
        // We fetch by URL, not by remote name, so a missing origin doesn't stop
        // us - and erroring would be unhelpful for a checkout that works fine.
        assert_eq!(
            plan(State::GitClone { remote: None }, true, UPSTREAM).unwrap(),
            Plan::Refresh
        );
    }

    #[test]
    fn defaults_come_from_the_environment_the_dagger_build_uses() {
        // Same variables TransitlandAtlas() reads, so a pinned atlas means the
        // same thing through either path.
        assert_eq!(REPO_ENV_VAR, "HEADWAY_TRANSITLAND_ATLAS_URL");
        assert_eq!(REF_ENV_VAR, "HEADWAY_TRANSITLAND_ATLAS_REF");
    }
}
