//! Giving every feed a `feed_id` that's unique across a transit zone.
//!
//! OpenTripPlanner takes the `feed_id` from `feed_info.txt` and uses it as the
//! key that joins a GTFS-RT stream to its static feed. Agencies pick these
//! themselves, so collisions across feeds in one zone are entirely possible,
//! and some feeds omit `feed_info.txt` altogether. We therefore always
//! overwrite it with an id derived from the Onestop ID.
//!
//! Nothing inside a GTFS archive references `feed_id`, so rewriting it can't
//! break internal consistency.

use crate::Result;

/// The `feed_id` we stamp into a feed, e.g. `headway-f-c23-soundtransit`.
pub fn feed_id_for(onestop_id: &str) -> String {
    format!("headway-{onestop_id}")
}

/// Produces a `feed_info.txt` body carrying `feed_id`.
///
/// `existing` is the agency's own file, when it shipped one. Its other fields
/// are preserved; only `feed_id` is overwritten, and the fields OTP requires
/// are synthesized if absent.
pub fn rewrite_feed_info(existing: Option<&str>, feed_id: &str) -> Result<String> {
    let mut headers: Vec<String> = Vec::new();
    let mut row: Vec<String> = Vec::new();

    if let Some(existing) = existing {
        let mut reader = csv::Reader::from_reader(existing.as_bytes());
        headers = reader.headers()?.iter().map(str::to_owned).collect();

        // Aggregate feeds conventionally list several rows here. Nothing inside
        // the archive references them, so there's no way to tell which entities
        // came from which - OTP itself just takes the first, so we do too.
        if let Some(first) = reader.records().next() {
            row = first?.iter().map(str::to_owned).collect();
        }
    }

    // Pad a short row so header and row stay aligned.
    row.resize(headers.len(), String::new());

    let mut set =
        |name: &str, value: String, overwrite: bool| match headers.iter().position(|h| h == name) {
            Some(index) => {
                if overwrite || row[index].trim().is_empty() {
                    row[index] = value;
                }
            }
            None => {
                headers.push(name.to_owned());
                row.push(value);
            }
        };

    set("feed_id", feed_id.to_owned(), true);

    // Without these, OTP fails the build outright with
    // MissingRequiredFieldException.
    set(
        "feed_publisher_name",
        format!("Feed Publisher: {feed_id}"),
        false,
    );
    set(
        "feed_publisher_url",
        format!("https://0.0.0.0/missing/feed_publisher_url/feed-id/{feed_id}"),
        false,
    );
    // A guess, but a required field.
    set("feed_lang", "en".to_owned(), false);

    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(&headers)?;
    writer.write_record(&row)?;
    Ok(String::from_utf8(writer.into_inner()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(csv_text: &str) -> Vec<(String, String)> {
        let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
        let headers: Vec<String> = reader
            .headers()
            .unwrap()
            .iter()
            .map(str::to_owned)
            .collect();
        let row: Vec<String> = reader
            .records()
            .next()
            .unwrap()
            .unwrap()
            .iter()
            .map(str::to_owned)
            .collect();
        headers.into_iter().zip(row).collect()
    }

    #[test]
    fn synthesizes_a_whole_file_when_there_is_none() {
        let out = rewrite_feed_info(None, "headway-f-c23-soundtransit").unwrap();
        let fields = parse(&out);

        assert!(fields.contains(&(
            "feed_id".to_owned(),
            "headway-f-c23-soundtransit".to_owned()
        )));
        // The three OTP insists on.
        for required in ["feed_publisher_name", "feed_publisher_url", "feed_lang"] {
            let value = fields.iter().find(|(k, _)| k == required).map(|(_, v)| v);
            assert!(value.is_some_and(|v| !v.is_empty()), "missing {required}");
        }
    }

    #[test]
    fn overwrites_an_existing_feed_id_but_keeps_everything_else() {
        let existing = "feed_id,feed_publisher_name,feed_publisher_url,feed_lang,feed_version\n\
                        TMB,TMB,https://www.tmb.cat,ca,165012062023002\n";
        let out = rewrite_feed_info(Some(existing), "headway-f-sp-tmb").unwrap();
        let fields = parse(&out);

        assert!(fields.contains(&("feed_id".to_owned(), "headway-f-sp-tmb".to_owned())));
        assert!(fields.contains(&("feed_publisher_name".to_owned(), "TMB".to_owned())));
        assert!(fields.contains(&("feed_lang".to_owned(), "ca".to_owned())));
        assert!(fields.contains(&("feed_version".to_owned(), "165012062023002".to_owned())));
    }

    #[test]
    fn adds_feed_id_when_the_agency_omitted_the_column() {
        // e.g. Whatcom County Transit ships feed_info.txt with no feed_id.
        let existing = "feed_publisher_name,feed_publisher_url,feed_lang\n\
                        WTA,https://ridewta.com,en\n";
        let out = rewrite_feed_info(Some(existing), "headway-f-c28-wta").unwrap();
        let fields = parse(&out);

        assert!(fields.contains(&("feed_id".to_owned(), "headway-f-c28-wta".to_owned())));
        assert!(fields.contains(&("feed_publisher_name".to_owned(), "WTA".to_owned())));
    }

    #[test]
    fn fills_in_only_the_required_fields_that_are_blank() {
        let existing = "feed_id,feed_publisher_name,feed_lang\n\
                        x,,de\n";
        let out = rewrite_feed_info(Some(existing), "headway-f-x").unwrap();
        let fields = parse(&out);

        // Blank publisher name gets synthesized...
        assert!(fields.contains(&(
            "feed_publisher_name".to_owned(),
            "Feed Publisher: headway-f-x".to_owned()
        )));
        // ...but a populated lang is left alone rather than forced to "en".
        assert!(fields.contains(&("feed_lang".to_owned(), "de".to_owned())));
    }

    #[test]
    fn ignores_all_but_the_first_row_of_an_aggregate_feed() {
        let existing = "feed_id,feed_publisher_name,feed_lang\n\
                        a,First,en\n\
                        b,Second,fr\n";
        let out = rewrite_feed_info(Some(existing), "headway-f-agg").unwrap();
        let fields = parse(&out);

        assert!(fields.contains(&("feed_publisher_name".to_owned(), "First".to_owned())));
        assert_eq!(out.lines().count(), 2, "header plus exactly one row");
    }

    #[test]
    fn handles_a_header_only_feed_info() {
        let existing = "feed_publisher_name,feed_publisher_url,feed_lang\n";
        let out = rewrite_feed_info(Some(existing), "headway-f-empty").unwrap();
        let fields = parse(&out);
        assert!(fields.contains(&("feed_id".to_owned(), "headway-f-empty".to_owned())));
    }

    #[test]
    fn feed_id_is_derived_from_the_onestop_id() {
        assert_eq!(
            feed_id_for("f-c23-soundtransit"),
            "headway-f-c23-soundtransit"
        );
    }
}
