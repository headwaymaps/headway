//! Typed MapLibre expression trees.

use serde::Serialize;
use serde_json::{json, Value as Json};

/// An operation in a MapLibre expression.
#[derive(Clone, Copy, Debug)]
pub enum Operator {
    All,
    Case,
    Coalesce,
    Concat,
    Divide,
    Get,
    Has,
    GreaterEqual,
    Length,
    Let,
    LessEqual,
    Literal,
    Match,
    Min,
    NotEqual,
    ToString,
    Var,
}

impl Operator {
    fn name(self) -> &'static str {
        match self {
            Operator::All => "all",
            Operator::Case => "case",
            Operator::Coalesce => "coalesce",
            Operator::Concat => "concat",
            Operator::Divide => "/",
            Operator::Get => "get",
            Operator::Has => "has",
            Operator::GreaterEqual => ">=",
            Operator::Length => "length",
            Operator::Let => "let",
            Operator::LessEqual => "<=",
            Operator::Literal => "literal",
            Operator::Match => "match",
            Operator::Min => "min",
            Operator::NotEqual => "!=",
            Operator::ToString => "to-string",
            Operator::Var => "var",
        }
    }
}

/// A typed MapLibre expression that can be used for a style property or filter.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Expression(Json);

/// Builds a typed MapLibre expression using concise prefix notation.
///
/// For example, `expr![Match, [Get, "class"], "park", "#0a0", "transparent"]`.
#[macro_export]
macro_rules! expr {
    ($operator:ident $(, $operand:tt)* $(,)?) => {
        $crate::Expression::op(
            $crate::Operator::$operator,
            [$($crate::expr!(@operand $operand)),*],
        )
    };
    (@operand [$operator:ident $(, $operand:tt)*]) => {
        $crate::expr!($operator $(, $operand)*)
    };
    (@operand [$($value:expr),* $(,)?]) => {
        $crate::Expression::array([$($crate::Expression::literal($value)),*])
    };
    (@operand $value:expr) => {
        $crate::Expression::literal($value)
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
        assert_eq!(
            serde_json::to_value(crate::expr![
                Match,
                [Get, "class"],
                "park",
                "#0a0",
                "transparent"
            ])
            .unwrap(),
            json!(["match", ["get", "class"], "park", "#0a0", "transparent"])
        );
    }
}
