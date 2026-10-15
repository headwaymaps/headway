//! Values a paint or layout property can take: constants, zoom functions, expressions.

use crate::color::Color;
use crate::expression::Expression;
use serde_json::{json, Value as Json};

/// A zoom function: `{"base": .., "stops": [[zoom, value], ..]}`.
#[derive(Clone, Debug, PartialEq)]
pub struct Stops {
    base: Option<f64>,
    stops: Vec<(f64, Json)>,
}

/// A zoom function, written as the table it is: `zoom!(base: 1.2; 12 => 0.5, 20 => 10.0)`.
#[macro_export]
macro_rules! zoom {
    (base: $base:expr; $($zoom:expr => $value:expr),* $(,)?) => {
        $crate::zoom_with_base($base, [$(($zoom as f64, $value)),*])
    };
    ($($zoom:expr => $value:expr),* $(,)?) => {
        $crate::zoom([$(($zoom as f64, $value)),*])
    };
}

/// Interpolates `value` across zoom levels.
pub fn zoom<V: Into<Json>>(pairs: impl IntoIterator<Item = (f64, V)>) -> Stops {
    zoom_with_base(1.0, pairs)
}

/// Interpolates `value` across zoom levels with an exponential interpolation base.
#[doc(hidden)]
pub fn zoom_with_base<V: Into<Json>>(
    base: f64,
    pairs: impl IntoIterator<Item = (f64, V)>,
) -> Stops {
    Stops {
        base: (base != 1.0).then_some(base),
        stops: pairs
            .into_iter()
            .map(|(zoom, value)| (zoom, value.into()))
            .collect(),
    }
}

impl Stops {
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

impl From<Expression> for Property {
    fn from(expression: Expression) -> Property {
        Property::Constant(expression.into_json())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_macro_reads_like_a_zoom_table() {
        assert_eq!(
            crate::zoom!(base: 1.2; 10.0 => 0.5, 13.0 => 2.0).to_json(),
            zoom_with_base(1.2, [(10.0, 0.5), (13.0, 2.0)]).to_json()
        );
    }
}
