//! Cheap exclusion of feeds that clearly aren't in the area of interest.
//!
//! Discovery ultimately decides whether a feed belongs to a transit zone by
//! downloading it and measuring where its stops are. That's the only reliable
//! answer, because the atlas carries no bounding boxes and no country field.
//! But downloading all ~4000 GTFS feeds to find the couple of dozen near one
//! city is wasteful, so first we drop the ones we have positive evidence are
//! somewhere else.
//!
//! # Exclusion, never inclusion
//!
//! Every rule here fires only on evidence that a feed is *elsewhere*. A feed we
//! know nothing about is always kept and measured. That asymmetry is the whole
//! design, and it isn't fussiness:
//!
//! - Country is inferable for only 62% of GTFS feeds. Turning that around into
//!   "keep the feeds that look like they're in my country" drops a fifth of
//!   them - tested against 619 known-US feeds, such a rule kept 494 and
//!   silently lost 125.
//! - A wrongly excluded feed is invisible. It never reaches the output CSV, so
//!   nobody curating that file has any reason to suspect the agency exists.
//!
//! Every exclusion is therefore recorded with its reason (see [`Skip`]), and
//! the whole prefilter can be turned off, so a missing agency is diagnosable
//! rather than a mystery.

use crate::dmfr::Feed;
use crate::geom::Rect;

/// Why a feed was excluded without being measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// Its geohash, padded by a ring of neighbours, is nowhere near the area.
    GeohashFarAway { geohash: String },
    /// Tagged as coming from a national aggregator for another country.
    ForeignAggregator { tag: String, country: &'static str },
    /// The feed's domain has a country-code TLD outside the area's countries.
    ForeignTld { tld: String },
    /// Its Onestop ID names a US state or Canadian province that isn't nearby.
    DistantRegion { region: String },
}

impl std::fmt::Display for Skip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Skip::GeohashFarAway { geohash } => {
                write!(f, "geohash {geohash} is far from the area")
            }
            Skip::ForeignAggregator { tag, country } => {
                write!(f, "tagged {tag}, a {country} aggregator")
            }
            Skip::ForeignTld { tld } => write!(f, "domain TLD .{tld} is a foreign country"),
            Skip::DistantRegion { region } => write!(f, "id names region {region}"),
        }
    }
}

/// Tags that only ever appear on feeds bulk-imported from one country's
/// national aggregator. They say nothing about *where* in that country a feed
/// is, which is useless for inclusion but conclusive for exclusion.
const AGGREGATOR_TAGS: &[(&str, &str, &str)] = &[
    ("gtfs_data_jp_prefecture_id", "JP", "Japanese"),
    ("es_nap_fichero_id", "ES", "Spanish"),
];

/// TLDs that don't name a country.
const GENERIC_TLDS: &[&str] = &[
    "com", "org", "net", "info", "io", "gov", "edu", "biz", "app", "cloud", "xyz", "me", "co",
];

const US_STATES: &[&str] = &[
    "al", "ak", "az", "ar", "ca", "co", "ct", "de", "fl", "ga", "hi", "id", "il", "in", "ia", "ks",
    "ky", "la", "me", "md", "ma", "mi", "mn", "ms", "mo", "mt", "ne", "nv", "nh", "nj", "nm", "ny",
    "nc", "nd", "oh", "ok", "or", "pa", "ri", "sc", "sd", "tn", "tx", "ut", "vt", "va", "wa", "wv",
    "wi", "wy", "dc", "pr",
];

const CA_PROVINCES: &[&str] = &[
    "ab", "bc", "mb", "nb", "nl", "ns", "nt", "nu", "on", "pe", "qc", "sk", "yt",
];

/// What counts as "elsewhere" for one transit zone.
///
/// The region and country lists must be drawn generously - they should cover
/// anywhere an operator serving this area might plausibly be registered. A
/// Seattle zone has to keep British Columbia and `.ca`, because cross-border
/// operators are real.
///
/// An *empty* list disables the rules that depend on it, rather than putting
/// everything out of scope. Failing that way round matters: the caller that
/// forgets to pass countries gets a slower run, not a silently gutted one.
#[derive(Debug, Clone)]
pub struct Prefilter {
    area: Rect,
    /// Country codes (lowercase) whose feeds stay in scope. Empty disables the
    /// ccTLD and aggregator-tag rules.
    countries: Vec<String>,
    /// US state / Canadian province codes (lowercase) that stay in scope.
    /// Empty disables the region rule.
    regions: Vec<String>,
}

impl Prefilter {
    pub fn new(
        area: Rect,
        countries: impl IntoIterator<Item = String>,
        regions: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            area,
            countries: countries.into_iter().map(|c| c.to_lowercase()).collect(),
            regions: regions.into_iter().map(|r| r.to_lowercase()).collect(),
        }
    }

    /// Returns why this feed can be skipped, or None if it must be measured.
    pub fn skip_reason(&self, feed: &Feed) -> Option<Skip> {
        // A geohash is the strongest signal we have, so when there is one it
        // decides the matter on its own. Padded by a ring of neighbours it
        // touched the real service area in 99.8% of feeds we could check, so a
        // padded cell that misses the area is near-conclusive.
        if let Some(geohash) = feed.geohash() {
            return if geohash.cell_with_margin().intersects(&self.area) {
                None
            } else {
                Some(Skip::GeohashFarAway {
                    geohash: geohash.as_str().to_owned(),
                })
            };
        }

        if !self.countries.is_empty() {
            for (tag, code, adjective) in AGGREGATOR_TAGS {
                if feed.tags.contains_key(*tag) && !self.in_scope_country(code) {
                    return Some(Skip::ForeignAggregator {
                        tag: (*tag).to_owned(),
                        country: adjective,
                    });
                }
            }

            if let Some(tld) = feed.source_tld() {
                let tld = tld.to_lowercase();
                let names_a_country = tld.len() == 2 && !GENERIC_TLDS.contains(&tld.as_str());
                if names_a_country && !self.in_scope_country(&tld) {
                    return Some(Skip::ForeignTld { tld });
                }
            }
        }

        if !self.regions.is_empty() {
            if let Some(region) = self.trailing_region(feed) {
                if !self.regions.contains(&region) {
                    return Some(Skip::DistantRegion { region });
                }
            }
        }

        None
    }

    fn in_scope_country(&self, code: &str) -> bool {
        self.countries.iter().any(|c| c.eq_ignore_ascii_case(code))
    }

    /// A US state or Canadian province named at the end of the Onestop ID, as
    /// in `f-smart~ca~us` (California) or `f-denman~bc~ca` (British Columbia).
    ///
    /// Only trusted when the final segment is the matching country, so the
    /// `ca` in `f-something~ca` isn't read as California.
    fn trailing_region(&self, feed: &Feed) -> Option<String> {
        let id = feed.onestop_id()?;
        let parts: Vec<&str> = id.name_parts().collect();
        let [.., region, country] = parts.as_slice() else {
            return None;
        };

        let known = match *country {
            "us" => US_STATES,
            "ca" => CA_PROVINCES,
            _ => return None,
        };
        known.contains(region).then(|| (*region).to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dmfr::{Feed, Operator, Urls};
    use crate::geom::Point;

    use std::collections::BTreeMap;

    fn seattle() -> Prefilter {
        Prefilter::new(
            Rect::new(Point::new(-122.462, 47.394), Point::new(-122.005, 47.831)),
            ["us".to_owned(), "ca".to_owned()],
            ["wa", "or", "id", "bc"].map(str::to_owned),
        )
    }

    fn feed(id: &str, domain: &str) -> Feed {
        Feed {
            id: id.to_owned(),
            spec: "gtfs".to_owned(),
            urls: Urls::default(),
            operators: vec![],
            tags: BTreeMap::new(),
            authorization: None,
            source_domain: domain.to_owned(),
        }
    }

    #[test]
    fn keeps_a_feed_whose_geohash_is_in_the_area() {
        // c23 covers the Puget Sound region.
        assert_eq!(
            seattle().skip_reason(&feed("f-c23-kingcounty", "kingcounty.gov")),
            None
        );
    }

    #[test]
    fn skips_a_feed_whose_geohash_is_far_away() {
        let skip = seattle()
            .skip_reason(&feed("f-9q8y-sfmta", "511.org"))
            .unwrap();
        assert_eq!(
            skip,
            Skip::GeohashFarAway {
                geohash: "9q8y".to_owned()
            }
        );
    }

    #[test]
    fn a_geohash_near_the_area_wins_over_other_signals() {
        // Geohash says Puget Sound; the ~ny~us suffix says New York. The
        // geohash is the better evidence, and it keeps the feed.
        let mut f = feed("f-c23-someagency~ny~us", "example.jp");
        f.tags
            .insert("gtfs_data_jp_prefecture_id".to_owned(), "13".to_owned());
        assert_eq!(seattle().skip_reason(&f), None);
    }

    #[test]
    fn skips_foreign_aggregator_tags() {
        let mut f = feed("f-toeibus~gtfs~jp", "example.com");
        f.tags
            .insert("gtfs_data_jp_prefecture_id".to_owned(), "13".to_owned());
        assert_eq!(
            seattle().skip_reason(&f),
            Some(Skip::ForeignAggregator {
                tag: "gtfs_data_jp_prefecture_id".to_owned(),
                country: "Japanese",
            })
        );
    }

    #[test]
    fn keeps_an_aggregator_tag_for_an_in_scope_country() {
        let japan = Prefilter::new(
            Rect::new(Point::new(139.0, 35.0), Point::new(140.0, 36.0)),
            ["jp".to_owned()],
            Vec::<String>::new(),
        );
        let mut f = feed("f-toeibus~gtfs~jp", "example.com");
        f.tags
            .insert("gtfs_data_jp_prefecture_id".to_owned(), "13".to_owned());
        assert_eq!(japan.skip_reason(&f), None);
    }

    #[test]
    fn skips_foreign_cctlds() {
        assert_eq!(
            seattle().skip_reason(&feed("f-pol~regio~pl", "polregio.pl")),
            Some(Skip::ForeignTld {
                tld: "pl".to_owned()
            })
        );
    }

    #[test]
    fn keeps_generic_tlds() {
        // .com says nothing about where an operator is.
        assert_eq!(
            seattle().skip_reason(&feed("f-mystery~transit", "example.com")),
            None
        );
    }

    #[test]
    fn keeps_in_scope_cctlds() {
        // Seattle is close enough to British Columbia that .ca stays in scope.
        assert_eq!(
            seattle().skip_reason(&feed("f-denman~bc~ca", "bcferries.ca")),
            None
        );
    }

    #[test]
    fn skips_distant_states() {
        assert_eq!(
            seattle().skip_reason(&feed("f-altavista~va~us", "altavistava.gov")),
            Some(Skip::DistantRegion {
                region: "va".to_owned()
            })
        );
    }

    #[test]
    fn keeps_nearby_states_and_provinces() {
        assert_eq!(
            seattle().skip_reason(&feed("f-pahtopublicpassage~wa~us", "example.gov")),
            None
        );
        assert_eq!(
            seattle().skip_reason(&feed("f-denman~bc~ca", "example.gov")),
            None
        );
    }

    #[test]
    fn does_not_read_a_trailing_country_as_a_region() {
        // "f-frederictontransit~ca" ends in the country code with no province
        // before it - there's no region claim to act on.
        assert_eq!(
            seattle().skip_reason(&feed("f-frederictontransit~ca", "example.gov")),
            None
        );
    }

    #[test]
    fn keeps_feeds_with_no_evidence_at_all() {
        // The 38% that say nothing. These must be measured, never dropped.
        assert_eq!(
            seattle().skip_reason(&feed("f-santa~clarita", "example.com")),
            None
        );
        assert_eq!(
            seattle().skip_reason(&feed("f-sarkshipping", "example.com")),
            None
        );
    }

    #[test]
    fn empty_country_list_disables_country_rules() {
        // Not "nothing is in scope". A caller that passes no countries should
        // get a slower run, not one that quietly drops every foreign-TLD feed.
        let no_countries = Prefilter::new(
            Rect::new(Point::new(-122.462, 47.394), Point::new(-122.005, 47.831)),
            Vec::<String>::new(),
            Vec::<String>::new(),
        );

        assert_eq!(
            no_countries.skip_reason(&feed("f-pol~regio~pl", "polregio.pl")),
            None
        );

        let mut jp = feed("f-toeibus~gtfs~jp", "example.com");
        jp.tags
            .insert("gtfs_data_jp_prefecture_id".to_owned(), "13".to_owned());
        assert_eq!(no_countries.skip_reason(&jp), None);

        // The geohash rule still applies - it doesn't depend on the lists.
        assert!(matches!(
            no_countries.skip_reason(&feed("f-9q8y-sfmta", "511.org")),
            Some(Skip::GeohashFarAway { .. })
        ));
    }

    #[test]
    fn empty_region_list_disables_the_region_rule() {
        let no_regions = Prefilter::new(
            Rect::new(Point::new(-122.462, 47.394), Point::new(-122.005, 47.831)),
            ["us".to_owned()],
            Vec::<String>::new(),
        );
        assert_eq!(
            no_regions.skip_reason(&feed("f-altavista~va~us", "example.gov")),
            None
        );
    }

    #[test]
    fn operator_geohash_is_used_when_the_feed_has_none() {
        let mut f = feed("f-smart~ca~us", "example.com");
        f.operators.push(Operator {
            onestop_id: "o-9qc-smart".to_owned(),
            name: Some("SMART".to_owned()),
            short_name: None,
            website: None,
            associated_feeds: vec![],
            tags: BTreeMap::new(),
        });
        // 9qc is in northern California, nowhere near Seattle.
        assert!(matches!(
            seattle().skip_reason(&f),
            Some(Skip::GeohashFarAway { .. })
        ));
    }
}
