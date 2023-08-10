use gtfout::Result;

use std::io::{Read, Write};

enum AreBikesAllowed {
    Allowed,
    #[allow(dead_code)]
    Forbidden,
    Unknown,
}

impl AreBikesAllowed {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "1",
            Self::Forbidden => "2",
            Self::Unknown => "0",
        }
    }
}

pub fn main() -> Result<()> {
    let input = std::io::stdin();
    let output = std::io::stdout();
    assume_bikes_allowed(input, output)?;

    Ok(())
}

pub fn assume_bikes_allowed<W: Write>(input: impl Read, output: W) -> Result<()> {
    let mut csv_reader = csv::Reader::from_reader(input);
    let mut csv_writer = csv::Writer::from_writer(output);

    let mut headers = csv_reader.headers()?.clone();

    let bikes_allowed_idx = headers.iter().position(|field| field == "bikes_allowed");
    if bikes_allowed_idx.is_none() {
        headers.push_field("bikes_allowed");
    };

    csv_writer.write_record(&headers)?;
    let desired_default = AreBikesAllowed::Allowed.as_str();

    for row in csv_reader.records() {
        let mut record = row?;
        if let Some(bikes_allowed_idx) = bikes_allowed_idx {
            match record.get(bikes_allowed_idx) {
                Some(other)
                    if other.trim() == "" || other.trim() == AreBikesAllowed::Unknown.as_str() =>
                {
                    log::debug!("bikes_allowed field is empty or unknown");
                    record = record
                        .into_iter()
                        .enumerate()
                        .map(|(record_idx, field)| {
                            if record_idx == bikes_allowed_idx {
                                desired_default
                            } else {
                                field
                            }
                        })
                        .collect();
                }
                Some(other) => log::debug!("bikes_allowed field: {other}"),
                // Can we get to this or will CSV error on its own?
                None => unreachable!("record was missing field found in header"),
            }
        } else {
            log::debug!("no bikes_allowed field");
            record.push_field(AreBikesAllowed::Allowed.as_str());
        }
        csv_writer.write_record(&record)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn start_logging() {
        static INIT_LOGGING: std::sync::Once = std::sync::Once::new();
        INIT_LOGGING.call_once(pretty_env_logger::init);
    }

    fn run_test(input: &str, expected_output: &str) {
        start_logging();
        let mut output = vec![];
        assume_bikes_allowed(input.as_bytes(), &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).expect("valid encoding"),
            expected_output
        );
    }

    #[test]
    fn dont_touch_when_already_populated() {
        let input = "aaa,bikes_allowed
x1,1
x2,1
";
        let expected_output = input;
        run_test(input, expected_output);
    }

    #[test]
    fn add_field_when_missing() {
        let input = "aaa,foo
x1,y1
x2,y2
";

        let expected_output = "aaa,foo,bikes_allowed
x1,y1,1
x2,y2,1
";
        run_test(input, expected_output);
    }

    #[test]
    fn populate_field_when_empty() {
        let input = "aaa,foo,bikes_allowed
x1,y1,
x2,y2,
";

        let expected_output = "aaa,foo,bikes_allowed
x1,y1,1
x2,y2,1
";
        run_test(input, expected_output);
    }

    #[test]
    fn populate_field_when_marked_as_unknown() {
        let input = "aaa,foo,bikes_allowed
x1,y1,0
x2,y2,1
";

        let expected_output = "aaa,foo,bikes_allowed
x1,y1,1
x2,y2,1
";
        run_test(input, expected_output);
    }

    #[test]
    fn leave_field_when_marked_as_forbidden() {
        let input = "aaa,foo,bikes_allowed
x1,y1,2
x2,y2,1
";

        let expected_output = "aaa,foo,bikes_allowed
x1,y1,2
x2,y2,1
";
        run_test(input, expected_output);
    }
}
