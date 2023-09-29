#![feature(lazy_cell)]
#![feature(let_chains)]

use thiserror::Error;

pub mod engine;
mod precedence;

#[derive(Clone, Debug)]
pub struct Expression {
    pub(crate) nodes: Vec<Node>,
    pub(crate) required_input_length: usize,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ExpressionConstructionError {
    #[error("the expression is empty")]
    EmptyExpression,
    #[error("was unable to match a set of parens in expression")]
    UbalancedParens,
}

impl Expression {
    pub fn from_infix(expression: &[Node]) -> Result<Self, ExpressionConstructionError> {
        if expression.is_empty() {
            return Err(ExpressionConstructionError::EmptyExpression);
        }

        let mut result: Vec<Node> = Vec::with_capacity(expression.len());
        let mut stack: Vec<Node> = Vec::with_capacity(expression.len());

        for op in expression {
            match op {
                Node::Operand => result.push(*op),
                Node::LeftParens => stack.push(*op),
                Node::RightParens => {
                    while let Some(n) = stack.last() && *n != Node::LeftParens {
                        result.push(stack.pop().unwrap());
                    }

                    if let Some(n) = stack.last() && *n != Node::LeftParens {
                        return Err(ExpressionConstructionError::UbalancedParens);
                    }

                    match stack.last() {
                        Some(Node::LeftParens) => stack.pop().unwrap(),
                        _ => return Err(ExpressionConstructionError::UbalancedParens),
                    };
                }
                Node::Operator(_) => {
                    let prec = precedence::precedence(*op);
                    while let Some(n) = stack.last() && prec <= precedence::precedence(*n) {
                        result.push(stack.pop().unwrap());
                    }

                    stack.push(*op);
                }
            };
        }

        while let Some(n) = stack.pop() {
            if n == Node::LeftParens {
                return Err(ExpressionConstructionError::UbalancedParens);
            }
            result.push(n);
        }

        let required_input_length = result.iter().filter(|&n| n == &Node::Operand).count();

        Ok(Expression {
            nodes: result,
            required_input_length,
        })
    }

    pub fn to_scalar_engine(self) -> engine::scalar::Engine {
        engine::scalar::Engine::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Operand,
    LeftParens,
    RightParens,
    Operator(char),
}

impl Node {
    pub const fn add_op() -> Node {
        Node::Operator('+')
    }
    pub const fn sub_op() -> Node {
        Node::Operator('-')
    }
    pub const fn mul_op() -> Node {
        Node::Operator('*')
    }
    pub const fn div_op() -> Node {
        Node::Operator('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_simple_expression_nodes() -> Vec<Node> {
        vec![
            Node::Operand,
            Node::add_op(),
            Node::LeftParens,
            Node::Operand,
            Node::sub_op(),
            Node::Operand,
            Node::RightParens,
        ]
    }

    #[test]
    fn missing_end_parens() {
        let mut nodes = get_simple_expression_nodes();
        nodes.pop();

        let expression = Expression::from_infix(&nodes);
        assert!(expression.is_err());
        let err = expression.unwrap_err();
        assert_eq!(err, ExpressionConstructionError::UbalancedParens);
    }

    #[test]
    fn missing_start_parens() {
        let mut nodes = get_simple_expression_nodes();
        nodes.retain(|n| n != &Node::LeftParens);

        let expression = Expression::from_infix(&nodes);
        assert!(expression.is_err());
        let err = expression.unwrap_err();
        assert_eq!(err, ExpressionConstructionError::UbalancedParens);
    }

    #[test]
    fn scalar() {
        let nodes = get_simple_expression_nodes();

        let expression = Expression::from_infix(&nodes);
        assert!(expression.is_ok());
        let expression = expression.unwrap();

        let engine = expression.to_scalar_engine();
        let result = engine.evaluate(&[1.0, 2.0, 1.0]);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result, 2.0);
    }
}
