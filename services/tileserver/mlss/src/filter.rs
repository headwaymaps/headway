//! Legacy-syntax filters, which is what the openmaptiles styles use.

use serde_json::{json, Value as Json};

#[derive(Clone, Debug, PartialEq)]
pub struct Filter(Json);

/// The `$type` of a feature.
#[derive(Clone, Copy, Debug)]
pub enum Geometry {
    Point,
    LineString,
    Polygon,
}

impl Geometry {
    fn name(self) -> &'static str {
        match self {
            Geometry::Point => "Point",
            Geometry::LineString => "LineString",
            Geometry::Polygon => "Polygon",
        }
    }
}

impl Filter {
    pub fn eq(field: &str, value: impl Into<Json>) -> Filter {
        Filter(json!(["==", field, value.into()]))
    }

    pub fn ne(field: &str, value: impl Into<Json>) -> Filter {
        Filter(json!(["!=", field, value.into()]))
    }

    pub fn gt(field: &str, value: impl Into<Json>) -> Filter {
        Filter(json!([">", field, value.into()]))
    }

    pub fn gte(field: &str, value: impl Into<Json>) -> Filter {
        Filter(json!([">=", field, value.into()]))
    }

    pub fn lt(field: &str, value: impl Into<Json>) -> Filter {
        Filter(json!(["<", field, value.into()]))
    }

    pub fn lte(field: &str, value: impl Into<Json>) -> Filter {
        Filter(json!(["<=", field, value.into()]))
    }

    pub fn has(field: &str) -> Filter {
        Filter(json!(["has", field]))
    }

    pub fn not_has(field: &str) -> Filter {
        Filter(json!(["!has", field]))
    }

    pub fn is_in<V: Into<Json>>(field: &str, values: impl IntoIterator<Item = V>) -> Filter {
        Filter(Self::membership("in", field, values))
    }

    pub fn not_in<V: Into<Json>>(field: &str, values: impl IntoIterator<Item = V>) -> Filter {
        Filter(Self::membership("!in", field, values))
    }

    /// `in` against one value collapses to `==`, which is how the hand-written styles read.
    fn membership<V: Into<Json>>(
        operator: &str,
        field: &str,
        values: impl IntoIterator<Item = V>,
    ) -> Json {
        let mut parts = vec![json!(operator), json!(field)];
        parts.extend(values.into_iter().map(Into::into));
        Json::Array(parts)
    }

    pub fn geometry(geometry: Geometry) -> Filter {
        Filter::eq("$type", geometry.name())
    }

    pub fn all(filters: impl IntoIterator<Item = Filter>) -> Filter {
        let mut parts = vec![json!("all")];
        parts.extend(filters.into_iter().map(|filter| filter.0));
        Filter(Json::Array(parts))
    }

    pub fn any(filters: impl IntoIterator<Item = Filter>) -> Filter {
        let mut parts = vec![json!("any")];
        parts.extend(filters.into_iter().map(|filter| filter.0));
        Filter(Json::Array(parts))
    }

    /// Conjoins another clause, flattening nested `all`s.
    pub fn and(self, other: Filter) -> Filter {
        let clauses = |filter: Filter| match filter.0 {
            Json::Array(parts) if parts.first() == Some(&json!("all")) => parts[1..]
                .iter()
                .map(|part| Filter(part.clone()))
                .collect::<Vec<_>>(),
            other => vec![Filter(other)],
        };
        let mut all = clauses(self);
        all.extend(clauses(other));
        Filter::all(all)
    }

    pub fn raw(json: Json) -> Filter {
        Filter(json)
    }

    pub fn to_json(&self) -> Json {
        self.0.clone()
    }
}
