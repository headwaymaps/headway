//! Naming the credentials OpenTripPlanner substitutes at runtime.
//!
//! This is not how *we* get a credential - that's [`crate::feed_config`], a YAML
//! table read at build time. This is the other end: a generated
//! `router-config.json` refers to an authenticated realtime feed's credential as
//! `${SOME_NAME}`, and OTP fills it in from its own environment. The config file
//! is committed and ends up in a ConfigMap, so the value must never be in it.
//!
//! The two are deliberately separate. Ours is a build input keyed by Onestop ID;
//! this one has to survive being an environment variable name, which Onestop IDs
//! can't do unmangled.

/// Env var OTP expects to hold the credential for a feed.
///
/// Onestop IDs contain characters that aren't legal in shell variable names, so
/// everything but ASCII alphanumerics becomes an underscore:
/// `f-9q8y-sfmta` -> `HEADWAY_GTFS_API_KEY_F_9Q8Y_SFMTA`.
///
/// Lossy by nature - two IDs could collide - but it has to be, and the
/// alternative of a hash would be unreadable in a k8s Secret.
pub fn api_key_env_var(feed_id: &str) -> String {
    let slug: String = feed_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("HEADWAY_GTFS_API_KEY_{slug}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_name_is_shell_safe() {
        assert_eq!(
            api_key_env_var("f-9q8y-sfmta"),
            "HEADWAY_GTFS_API_KEY_F_9Q8Y_SFMTA"
        );
        assert_eq!(
            api_key_env_var("f-sf~bay~area~rg"),
            "HEADWAY_GTFS_API_KEY_F_SF_BAY_AREA_RG"
        );
        // Non-ASCII appears in real Onestop IDs, e.g. f-aléop~pays~de~la~loire.
        assert_eq!(
            api_key_env_var("f-aléop~pays"),
            "HEADWAY_GTFS_API_KEY_F_AL_OP_PAYS"
        );
    }
}
