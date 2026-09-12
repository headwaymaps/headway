//! The transitland-atlas: getting a clone onto disk, parsing its catalog, and
//! joining the realtime feeds to the static feeds they update.

pub mod dmfr;
pub mod realtime;

use crate::Result;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Our fork, unless [`REPO_ENV_VAR`] says otherwise.
///
/// The `headway` branch carries fixes to feeds we use that have not landed
/// upstream yet -- stale `authorization` blocks on feeds that are actually
/// public, and endpoints that moved. Each is a self-contained commit destined
/// for an upstream PR, so this should return to upstream/main once they merge.
pub const DEFAULT_REPO: &str = "https://github.com/michaelkirk/transitland-atlas.git";

/// The ref to track, unless [`REF_ENV_VAR`] says otherwise.
pub const DEFAULT_REF: &str = "headway";

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
    /// Deliberately not a validity check - whether the directory really holds an
    /// atlas is [`dmfr::load_catalog`]'s call, and its error is better.
    pub fn ensure(&self) -> Result<&Path> {
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
    GitClone {
        remote: Option<String>,
    },
    OtherContent,
}

#[derive(Debug, PartialEq, Eq)]
enum Plan {
    Use,
    Clone,
    Refresh,
}

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
        (State::GitClone { .. } | State::OtherContent, false) => Ok(Plan::Use),

        (State::MissingOrEmpty, false) => Err(format!(
            "no transitland-atlas clone at that path - clone one there, or drop --no-download to fetch {repo}"
        )
        .into()),

        (State::MissingOrEmpty, true) => Ok(Plan::Clone),

        (State::GitClone { remote }, true) => {
            match remote {
                Some(remote) if !same_repo(&remote, repo) => Err(format!(
                    "refusing to refresh an existing clone of {remote}, which is not {repo} - remove it, or point --atlas-path somewhere else"
                )
                .into()),
                _ => Ok(Plan::Refresh),
            }
        }

        (State::OtherContent, true) => Err(
            "refusing to clone over a directory that already has content and is not a git clone - remove it, or point --atlas-path somewhere else"
                .to_owned()
                .into(),
        ),
    }
}

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
    fn plans() {
        assert_eq!(
            plan(git_clone(UPSTREAM), false, UPSTREAM).unwrap(),
            Plan::Use
        );
        assert_eq!(
            plan(git_clone("https://example.com/fork"), false, UPSTREAM).unwrap(),
            Plan::Use
        );
        assert_eq!(
            plan(State::OtherContent, false, UPSTREAM).unwrap(),
            Plan::Use
        );

        assert_eq!(
            plan(State::MissingOrEmpty, true, UPSTREAM).unwrap(),
            Plan::Clone
        );

        assert_eq!(
            plan(git_clone(UPSTREAM), true, UPSTREAM).unwrap(),
            Plan::Refresh
        );
        assert_eq!(
            plan(State::GitClone { remote: None }, true, UPSTREAM).unwrap(),
            Plan::Refresh
        );

        assert!(plan(State::OtherContent, true, UPSTREAM).is_err());
    }

    #[test]
    fn errors_say_what_to_do_about_them() {
        let missing = plan(State::MissingOrEmpty, false, UPSTREAM)
            .unwrap_err()
            .to_string();
        assert!(missing.contains("--no-download"), "{missing}");

        let wrong_repo = plan(git_clone("https://example.com/other"), true, UPSTREAM)
            .unwrap_err()
            .to_string();
        assert!(wrong_repo.contains("example.com/other"), "{wrong_repo}");
    }

    #[test]
    fn the_read_only_path_never_looks_up_the_remote() {
        let temp = std::env::temp_dir().join(format!("gtfout-atlas-{}", std::process::id()));
        std::fs::create_dir_all(temp.join(".git")).unwrap();
        std::fs::create_dir_all(temp.join("feeds")).unwrap();

        let state = state_of(&temp, false).unwrap();
        assert_eq!(state, State::GitClone { remote: None });
        assert_eq!(plan(state, false, UPSTREAM).unwrap(), Plan::Use);

        std::fs::remove_dir_all(&temp).ok();
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
        assert!(!same_repo("https://example.com/a", "https://example.com/b"));
        assert!(!same_repo(
            "https://github.com/someone/transitland-atlas",
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
}
