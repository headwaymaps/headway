//! Values a paint or layout property can take: constants, zoom functions, expressions.

use crate::color::Color;
use serde_json::{json, Value as Json};

/// A zoom function: `{"base": .., "stops": [[zoom, value], ..]}`.
#[derive(Clone, Debug, PartialEq)]
pub struct Stops {
    base: Option<f64>,
    stops: Vec<(f64, Json)>,
}

/// A zoom function, written as the table it is: `zoom!(base 1.2; 12 => 0.5, 20 => 10.0)`.
#[macro_export]
macro_rules! zoom {
    (base $base:expr; $($zoom:expr => $value:expr),* $(,)?) => {
        $crate::stops([$(($zoom as f64, $value)),*]).base($base)
    };
    ($($zoom:expr => $value:expr),* $(,)?) => {
        $crate::stops([$(($zoom as f64, $value)),*])
    };
}

/// Interpolates `value` across zoom levels.
pub fn stops<V: Into<Json>>(pairs: impl IntoIterator<Item = (f64, V)>) -> Stops {
    Stops {
        base: None,
        stops: pairs
            .into_iter()
            .map(|(zoom, value)| (zoom, value.into()))
            .collect(),
    }
}

impl Stops {
    /// The exponential base of the interpolation; maplibre defaults to 1 (linear).
    pub fn base(mut self, base: f64) -> Self {
        self.base = Some(base);
        self
    }

    fn to_json(&self) -> Json {
        let stops: Vec<Json> = self
            .stops
            .iter()
            .map(|(zoom, value)| json!([zoom, value]))
            .collect();
        match self.base {
            Some(base) => json!({ "base": base, "stops": stops }),
            None => json!({ "stops": stops }),
        }
    }
}

/// The value of a single paint or layout property.
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    Constant(Json),
    Stops(Stops),
}

impl Property {
    pub fn to_json(&self) -> Json {
        match self {
            Property::Constant(value) => value.clone(),
            Property::Stops(stops) => stops.to_json(),
        }
    }

    /// Applies `scale` to every number in the property, zoom stops included.
    pub fn map_numbers(&self, scale: impl Fn(f64) -> f64) -> Property {
        let apply = |value: &Json| match value.as_f64() {
            Some(number) => json!(scale(number)),
            None => value.clone(),
        };
        match self {
            Property::Constant(value) => Property::Constant(apply(value)),
            Property::Stops(stops) => Property::Stops(Stops {
                base: stops.base,
                stops: stops
                    .stops
                    .iter()
                    .map(|(zoom, value)| (*zoom, apply(value)))
                    .collect(),
            }),
        }
    }
}

impl From<Stops> for Property {
    fn from(stops: Stops) -> Property {
        Property::Stops(stops)
    }
}

impl From<Color> for Property {
    fn from(color: Color) -> Property {
        Property::Constant(color.into())
    }
}

impl From<&Color> for Property {
    fn from(color: &Color) -> Property {
        Property::Constant(color.clone().into())
    }
}

macro_rules! constant_from {
    ($($type:ty),*) => {
        $(impl From<$type> for Property {
            fn from(value: $type) -> Property {
                Property::Constant(json!(value))
            }
        })*
    };
}

constant_from!(
    f64,
    i32,
    bool,
    &str,
    String,
    Json,
    Vec<f64>,
    Vec<&str>,
    Vec<String>,
    [f64; 2],
    [f64; 4]
);
