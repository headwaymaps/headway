use crate::TravelMode;
use serde::de::Visitor;
use serde::{de, de::IntoDeserializer, Deserialize, Deserializer};
use std::fmt;

// Comma separated list of travel modes
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TravelModes(Vec<TravelMode>);
impl TravelModes {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn first(&self) -> Option<&TravelMode> {
        self.0.first()
    }
}

impl<'de> Deserialize<'de> for TravelModes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CommaSeparatedVecVisitor;

        impl Visitor<'_> for CommaSeparatedVecVisitor {
            type Value = TravelModes;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a comma-separated string")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                let modes = value
                    .split(',')
                    .map(|s| TravelMode::deserialize(s.into_deserializer()))
                    .collect::<std::result::Result<_, _>>()?;
                Ok(TravelModes(modes))
            }
        }

        deserializer.deserialize_str(CommaSeparatedVecVisitor)
    }
}
