//! Typed MapLibre expression trees.

use serde::Serialize;
use serde_json::{json, Value as Json};

/// An operation in a MapLibre expression.
#[derive(Clone, Copy, Debug)]
pub enum Operator {
    Add,
    All,
    Case,
    Coalesce,
    Concat,
    Divide,
    Equal,
    Get,
    Has,
    GreaterEqual,
    Interpolate,
    Length,
    Let,
    LessEqual,
    Literal,
    Match,
    Max,
    Min,
    Multiply,
    NotEqual,
    ToString,
    Var,
    Zoom,
}

impl Operator {
    fn name(self) -> &'static str {
        match self {
            Operator::Add => "+",
            Operator::All => "all",
            Operator::Case => "case",
            Operator::Coalesce => "coalesce",
            Operator::Concat => "concat",
            Operator::Divide => "/",
            Operator::Equal => "==",
            Operator::Get => "get",
            Operator::Has => "has",
            Operator::GreaterEqual => ">=",
            Operator::Interpolate => "interpolate",
            Operator::Length => "length",
            Operator::Let => "let",
            Operator::LessEqual => "<=",
            Operator::Literal => "literal",
            Operator::Match => "match",
            Operator::Max => "max",
            Operator::Min => "min",
            Operator::Multiply => "*",
            Operator::NotEqual => "!=",
            Operator::ToString => "to-string",
            Operator::Var => "var",
            Operator::Zoom => "zoom",
        }
    }
}

/// A typed MapLibre expression that can be used for a style property or filter.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Expression(Json);

/// Builds a MapLibre expression in MapLibre's own syntax, e.g.
/// `expr!(["match", ["get", "class"], "park", (green), "transparent"])`.
///
/// An array starting with an operator name is a call; any other array is a literal array.
/// Strings, numbers, and booleans are literals, and a bare variable or a parenthesized Rust
/// expression is spliced in.
#[macro_export]
macro_rules! expr {
    ([$($item:tt),* $(,)?]) => {
        $crate::expr!(@array [$($item),*])
    };
    (@array [$first:tt $(, $rest:tt)*]) => {
        $crate::expr!(@call $first [$($rest),*])
    };
    (@array []) => {
        $crate::Expression::array([])
    };
    (@call "+" [$($operand:tt),*]) => { $crate::expr!(@op Add [$($operand),*]) };
    (@call "all" [$($operand:tt),*]) => { $crate::expr!(@op All [$($operand),*]) };
    (@call "case" [$($operand:tt),*]) => { $crate::expr!(@op Case [$($operand),*]) };
    (@call "coalesce" [$($operand:tt),*]) => { $crate::expr!(@op Coalesce [$($operand),*]) };
    (@call "concat" [$($operand:tt),*]) => { $crate::expr!(@op Concat [$($operand),*]) };
    (@call "/" [$($operand:tt),*]) => { $crate::expr!(@op Divide [$($operand),*]) };
    (@call "==" [$($operand:tt),*]) => { $crate::expr!(@op Equal [$($operand),*]) };
    (@call "get" [$($operand:tt),*]) => { $crate::expr!(@op Get [$($operand),*]) };
    (@call "has" [$($operand:tt),*]) => { $crate::expr!(@op Has [$($operand),*]) };
    (@call ">=" [$($operand:tt),*]) => { $crate::expr!(@op GreaterEqual [$($operand),*]) };
    (@call "interpolate" [$($operand:tt),*]) => { $crate::expr!(@op Interpolate [$($operand),*]) };
    (@call "length" [$($operand:tt),*]) => { $crate::expr!(@op Length [$($operand),*]) };
    (@call "let" [$($operand:tt),*]) => { $crate::expr!(@op Let [$($operand),*]) };
    (@call "<=" [$($operand:tt),*]) => { $crate::expr!(@op LessEqual [$($operand),*]) };
    (@call "literal" [$($operand:tt),*]) => { $crate::expr!(@op Literal [$($operand),*]) };
    (@call "match" [$($operand:tt),*]) => { $crate::expr!(@op Match [$($operand),*]) };
    (@call "max" [$($operand:tt),*]) => { $crate::expr!(@op Max [$($operand),*]) };
    (@call "min" [$($operand:tt),*]) => { $crate::expr!(@op Min [$($operand),*]) };
    (@call "*" [$($operand:tt),*]) => { $crate::expr!(@op Multiply [$($operand),*]) };
    (@call "!=" [$($operand:tt),*]) => { $crate::expr!(@op NotEqual [$($operand),*]) };
    (@call "to-string" [$($operand:tt),*]) => { $crate::expr!(@op ToString [$($operand),*]) };
    (@call "var" [$($operand:tt),*]) => { $crate::expr!(@op Var [$($operand),*]) };
    (@call "zoom" [$($operand:tt),*]) => { $crate::expr!(@op Zoom [$($operand),*]) };
    (@call $first:tt [$($rest:tt),*]) => {
        $crate::Expression::array([$crate::expr!(@operand $first) $(, $crate::expr!(@operand $rest))*])
    };
    (@op $operator:ident [$($operand:tt),*]) => {
        $crate::Expression::op($crate::Operator::$operator, [$($crate::expr!(@operand $operand)),*])
    };
    (@operand [$($item:tt),*]) => {
        $crate::expr!(@array [$($item),*])
    };
    (@operand $value:tt) => {
        $crate::Expression::from($value)
    };
}

impl Expression {
    /// Builds an expression from a typed MapLibre operator and its operands.
    pub fn op(operator: Operator, operands: impl IntoIterator<Item = Expression>) -> Self {
        let mut expression = vec![json!(operator.name())];
        expression.extend(operands.into_iter().map(|operand| operand.0));
        Self(Json::Array(expression))
    }

    /// A literal JSON value used as an expression operand.
    pub fn literal(value: impl Into<Json>) -> Self {
        Self(value.into())
    }

    /// A literal array, useful for `match` labels and `literal` operands.
    pub fn array(values: impl IntoIterator<Item = Expression>) -> Self {
        Self(Json::Array(
            values.into_iter().map(|value| value.0).collect(),
        ))
    }

    pub(crate) fn into_inner(self) -> Json {
        self.0
    }
}

impl From<crate::Color> for Expression {
    fn from(color: crate::Color) -> Self {
        Self::literal(color)
    }
}

impl From<&str> for Expression {
    fn from(value: &str) -> Self {
        Self::literal(value)
    }
}

impl From<String> for Expression {
    fn from(value: String) -> Self {
        Self::literal(value)
    }
}

impl From<f64> for Expression {
    fn from(value: f64) -> Self {
        Self::literal(value)
    }
}

impl From<i32> for Expression {
    fn from(value: i32) -> Self {
        Self::literal(value)
    }
}

impl From<bool> for Expression {
    fn from(value: bool) -> Self {
        Self::literal(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_typed_operators() {
        assert_eq!(
            serde_json::to_value(Expression::op(
                Operator::Match,
                [
                    Expression::op(Operator::Get, ["class".into()]),
                    "park".into(),
                    "#0a0".into(),
                    "transparent".into(),
                ],
            ))
            .unwrap(),
            json!(["match", ["get", "class"], "park", "#0a0", "transparent"])
        );
    }

    #[test]
    fn macro_reads_like_an_expression() {
        let body = crate::expr!(["get", "ref"]);
        assert_eq!(
            serde_json::to_value(crate::expr!([
                "let",
                "ref",
                ["coalesce", ["get", "route_1_ref"], body],
                ["literal", [0, 0.11]]
            ]))
            .unwrap(),
            json!([
                "let",
                "ref",
                ["coalesce", ["get", "route_1_ref"], ["get", "ref"]],
                ["literal", [0, 0.11]]
            ])
        );
    }
}
