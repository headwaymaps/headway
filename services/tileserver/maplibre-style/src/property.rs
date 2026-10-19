//! Values a paint or layout property can take: constants, zoom functions, expressions.

use crate::color::Color;
use crate::expression::Expression;
use serde::Serialize;
use serde_json::{json, Value as Json};

/// A zoom function: `{"base": .., "stops": [[zoom, value], ..]}`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stops<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<f64>,
    stops: Vec<(f64, T)>,
}

impl<T> Stops<T> {
    /// Transforms each stop's value, keeping the zooms and interpolation.
    pub fn map<U>(&self, change: impl Fn(&T) -> U) -> Stops<U> {
        Stops {
            base: self.base,
            stops: self
                .stops
                .iter()
                .map(|(zoom, value)| (*zoom, change(value)))
                .collect(),
        }
    }
}

impl<T: Serialize> Stops<T> {
    fn into_json(self) -> Stops<Json> {
        self.map(|value| serde_json::to_value(value).expect("stop values serialize"))
    }
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
pub fn zoom<V>(pairs: impl IntoIterator<Item = (f64, V)>) -> Stops<V> {
    zoom_with_base(1.0, pairs)
}

/// Interpolates `value` across zoom levels with an exponential interpolation base.
#[doc(hidden)]
pub fn zoom_with_base<V>(base: f64, pairs: impl IntoIterator<Item = (f64, V)>) -> Stops<V> {
    Stops {
        base: (base != 1.0).then_some(base),
        stops: pairs.into_iter().collect(),
    }
}

/// The value of a paint or layout property: a constant, a zoom function, or an expression.
///
/// Typed setters take `Property<Color>` or `Property<f64>`; layers store the untyped
/// `Property<Json>`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Property<T> {
    Constant(T),
    Stops(Stops<T>),
    Expression(Expression),
}

impl<T> Property<T> {
    /// Transforms each value, keeping any zoom function's shape.
    pub fn map(&self, change: impl Fn(&T) -> T) -> Property<T> {
        match self {
            Property::Constant(value) => Property::Constant(change(value)),
            Property::Stops(stops) => Property::Stops(stops.map(change)),
            Property::Expression(expression) => {
                panic!("can't derive values from an expression: {expression:?}")
            }
        }
    }
}

impl<T: Serialize> Property<T> {
    pub(crate) fn into_json(self) -> Property<Json> {
        match self {
            Property::Constant(value) => {
                Property::Constant(serde_json::to_value(value).expect("property values serialize"))
            }
            Property::Stops(stops) => Property::Stops(stops.into_json()),
            Property::Expression(expression) => Property::Expression(expression),
        }
    }
}

impl<T> From<Expression> for Property<T> {
    fn from(expression: Expression) -> Property<T> {
        Property::Expression(expression)
    }
}

macro_rules! typed_from {
    ($($type:ty),*) => {
        $(
            impl From<$type> for Property<$type> {
                fn from(value: $type) -> Property<$type> {
                    Property::Constant(value)
                }
            }

            impl From<Stops<$type>> for Property<$type> {
                fn from(stops: Stops<$type>) -> Property<$type> {
                    Property::Stops(stops)
                }
            }
        )*
    };
}

typed_from!(Color, f64);

impl From<&Color> for Property<Color> {
    fn from(color: &Color) -> Property<Color> {
        Property::Constant(color.clone())
    }
}

/// Parses CSS, panicking on anything that isn't a color.
impl From<&str> for Property<Color> {
    fn from(css: &str) -> Property<Color> {
        Property::Constant(Color::parse(css))
    }
}

impl<T: Serialize> From<Stops<T>> for Property<Json> {
    fn from(stops: Stops<T>) -> Property<Json> {
        Property::Stops(stops.into_json())
    }
}

macro_rules! untyped_from {
    ($($type:ty),*) => {
        $(impl From<$type> for Property<Json> {
            fn from(value: $type) -> Property<Json> {
                Property::Constant(json!(value))
            }
        })*
    };
}

untyped_from!(
    Color,
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
            serde_json::to_value(crate::zoom!(base: 1.2; 10.0 => 0.5, 13.0 => 2.0)).unwrap(),
            serde_json::to_value(zoom_with_base(1.2, [(10.0, 0.5), (13.0, 2.0)])).unwrap()
        );
    }
}
