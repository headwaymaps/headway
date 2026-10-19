//! Legacy-syntax filters, which is what the openmaptiles styles use.

use crate::expression::Expression;
use serde::Serialize;
use serde_json::{json, Value as Json};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Filter(Json);

impl Filter {
    /// Matches a feature's OpenMapTiles `class`.
    pub fn class(value: impl Into<Json>) -> Filter {
        Filter::eq("class", value)
    }

    /// Matches a feature whose OpenMapTiles `class` is one of `values`.
    pub fn class_in<V: Into<Json>>(values: impl IntoIterator<Item = V>) -> Filter {
        Filter::is_in("class", values)
    }

    /// Excludes a feature with this OpenMapTiles `class`.
    pub fn class_not(value: impl Into<Json>) -> Filter {
        Filter::ne("class", value)
    }

    /// Excludes features whose OpenMapTiles `class` is one of `values`.
    pub fn class_not_in<V: Into<Json>>(values: impl IntoIterator<Item = V>) -> Filter {
        Filter::not_in("class", values)
    }

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

    pub fn point() -> Filter {
        Filter::eq("$type", "Point")
    }

    pub fn linestring() -> Filter {
        Filter::eq("$type", "LineString")
    }

    pub fn polygon() -> Filter {
        Filter::eq("$type", "Polygon")
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

    pub fn and_eq(self, field: &str, value: impl Into<Json>) -> Filter {
        self.and(Filter::eq(field, value))
    }
    pub fn and_ne(self, field: &str, value: impl Into<Json>) -> Filter {
        self.and(Filter::ne(field, value))
    }
    pub fn and_gt(self, field: &str, value: impl Into<Json>) -> Filter {
        self.and(Filter::gt(field, value))
    }
    pub fn and_gte(self, field: &str, value: impl Into<Json>) -> Filter {
        self.and(Filter::gte(field, value))
    }
    pub fn and_lt(self, field: &str, value: impl Into<Json>) -> Filter {
        self.and(Filter::lt(field, value))
    }
    pub fn and_lte(self, field: &str, value: impl Into<Json>) -> Filter {
        self.and(Filter::lte(field, value))
    }
    pub fn and_has(self, field: &str) -> Filter {
        self.and(Filter::has(field))
    }
    pub fn and_not_has(self, field: &str) -> Filter {
        self.and(Filter::not_has(field))
    }
    pub fn and_in<V: Into<Json>>(self, field: &str, values: impl IntoIterator<Item = V>) -> Filter {
        self.and(Filter::is_in(field, values))
    }
    pub fn and_not_in<V: Into<Json>>(
        self,
        field: &str,
        values: impl IntoIterator<Item = V>,
    ) -> Filter {
        self.and(Filter::not_in(field, values))
    }
    pub fn raw(json: Json) -> Filter {
        Filter(json)
    }

    /// Uses a typed MapLibre expression as a filter clause.
    pub fn expression(expression: Expression) -> Filter {
        Filter(expression.into_inner())
    }
}

/// Class-specific filter clauses for fluent style definitions.
pub trait ClassFilter {
    /// Conjoins a matching OpenMapTiles `class` clause.
    fn class(self, value: impl Into<Json>) -> Filter;

    /// Conjoins a matching OpenMapTiles `class` membership clause.
    fn class_in<V: Into<Json>>(self, values: impl IntoIterator<Item = V>) -> Filter;

    /// Conjoins an excluding OpenMapTiles `class` clause.
    fn class_not(self, value: impl Into<Json>) -> Filter;

    /// Conjoins an excluding OpenMapTiles `class` membership clause.
    fn class_not_in<V: Into<Json>>(self, values: impl IntoIterator<Item = V>) -> Filter;
}

impl ClassFilter for Filter {
    fn class(self, value: impl Into<Json>) -> Filter {
        self.and(Filter::class(value))
    }

    fn class_in<V: Into<Json>>(self, values: impl IntoIterator<Item = V>) -> Filter {
        self.and(Filter::class_in(values))
    }

    fn class_not(self, value: impl Into<Json>) -> Filter {
        self.and(Filter::class_not(value))
    }

    fn class_not_in<V: Into<Json>>(self, values: impl IntoIterator<Item = V>) -> Filter {
        self.and(Filter::class_not_in(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn class_clauses_are_available_as_constructors_and_chains() {
        assert_eq!(
            serde_json::to_value(Filter::class_in(["park", "cemetery"])).unwrap(),
            json!(["in", "class", "park", "cemetery"])
        );
        assert_eq!(
            serde_json::to_value(Filter::polygon().class("park")).unwrap(),
            json!(["all", ["==", "$type", "Polygon"], ["==", "class", "park"]])
        );
        assert_eq!(
            serde_json::to_value(Filter::polygon().class_not_in(["park", "cemetery"]),).unwrap(),
            json!([
                "all",
                ["==", "$type", "Polygon"],
                ["!in", "class", "park", "cemetery"]
            ])
        );
    }
}
