//! Onestop IDs, the identifiers Transitland Atlas uses for feeds and operators.
//!
//! A Onestop ID has two or three hyphen-separated components: an entity prefix
//! (`f-` for feeds, `o-` for operators), an *optional* geohash, and a name in
//! which `~` stands in for punctuation. So both of these are well-formed:
//!
//! ```text
//! f-9q8y-sfmta          three-part, geohash "9q8y"
//! f-smart~ca~us         two-part, no geohash
//! ```
//!
//! The geohash was mandatory in Transitland v1 and optional in v2, which is
//! why only about a third of the catalog carries one today - and why the newest
//! feeds almost never do. See `Geohash` for what the geohash does and doesn't
//! tell you about where a feed is.

use crate::geohash::Geohash;

/// A parsed Onestop ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnestopId {
    prefix: String,
    geohash: Option<Geohash>,
    name: String,
}

impl OnestopId {
    /// Parses a Onestop ID, or returns None if it doesn't have a recognizable
    /// `<prefix>-<name>` shape.
    ///
    /// The middle component is only read as a geohash when it's plausibly one:
    /// base32 and no longer than [`Geohash::MAX_LEN`]. Anything else is treated
    /// as part of the name, since a two-part ID's name may itself contain
    /// hyphens (`f-f-viarail~traindecharlevoix` is a real example, whose name
    /// component begins with a bare "f").
    pub fn parse(id: &str) -> Option<Self> {
        let (prefix, rest) = id.split_once('-')?;
        if prefix.is_empty() || rest.is_empty() {
            return None;
        }

        if let Some((maybe_geohash, name)) = rest.split_once('-') {
            if !name.is_empty() {
                if let Some(geohash) = Geohash::parse(maybe_geohash) {
                    return Some(Self {
                        prefix: prefix.to_owned(),
                        geohash: Some(geohash),
                        name: name.to_owned(),
                    });
                }
            }
        }

        Some(Self {
            prefix: prefix.to_owned(),
            geohash: None,
            name: rest.to_owned(),
        })
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn geohash(&self) -> Option<&Geohash> {
        self.geohash.as_ref()
    }

    /// The name component, with `~` still in place.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name split on `~`, e.g. `["smart", "ca", "us"]`.
    ///
    /// Contributors often encode a region in the trailing segments, which is
    /// the only locality hint many two-part IDs have.
    pub fn name_parts(&self) -> impl Iterator<Item = &str> {
        self.name.split('~')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_part_id() {
        let id = OnestopId::parse("f-9q8y-sfmta").unwrap();
        assert_eq!(id.prefix(), "f");
        assert_eq!(id.geohash().unwrap().as_str(), "9q8y");
        assert_eq!(id.name(), "sfmta");
    }

    #[test]
    fn two_part_id() {
        let id = OnestopId::parse("f-smart~ca~us").unwrap();
        assert_eq!(id.prefix(), "f");
        assert_eq!(id.geohash(), None);
        assert_eq!(id.name(), "smart~ca~us");
        assert_eq!(id.name_parts().collect::<Vec<_>>(), ["smart", "ca", "us"]);
    }

    #[test]
    fn middle_component_that_isnt_a_geohash_stays_in_the_name() {
        // "a" is not a base32 geohash character, so this is a two-part ID whose
        // name happens to contain a hyphen.
        let id = OnestopId::parse("f-transit-authority").unwrap();
        assert_eq!(id.geohash(), None);
        assert_eq!(id.name(), "transit-authority");
    }

    #[test]
    fn overlong_middle_component_is_not_a_geohash() {
        // Longer than any geohash Transitland actually uses; much more likely
        // to be a name that happens to be base32-clean.
        let id = OnestopId::parse("f-bcdefghjkmn-someagency").unwrap();
        assert_eq!(id.geohash(), None);
        assert_eq!(id.name(), "bcdefghjkmn-someagency");
    }

    #[test]
    fn single_char_middle_component_reads_as_a_geohash() {
        // A real ID where the middle component is a single base32 character.
        // We can't tell "precision-1 geohash" from "name that starts with a
        // hyphenated letter", so we take it as a geohash. That's the safe way
        // round: a precision-1 cell is ~5000km across, so it's too coarse to
        // wrongly exclude anything, and the prefilter is the only consumer.
        let id = OnestopId::parse("f-f-viarail~traindecharlevoix").unwrap();
        assert_eq!(id.geohash().map(Geohash::as_str), Some("f"));
        assert_eq!(id.name(), "viarail~traindecharlevoix");
    }

    #[test]
    fn operator_ids_parse_too() {
        let id = OnestopId::parse("o-9q9-actransit").unwrap();
        assert_eq!(id.prefix(), "o");
        assert_eq!(id.geohash().unwrap().as_str(), "9q9");
    }

    #[test]
    fn malformed() {
        assert_eq!(OnestopId::parse(""), None);
        assert_eq!(OnestopId::parse("f-"), None);
        assert_eq!(OnestopId::parse("nohyphen"), None);
    }
}
