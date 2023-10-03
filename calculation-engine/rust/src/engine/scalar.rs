use std::result::Result;
use tinyvec::tiny_vec;

use crate::engine::EvaluationError;
use crate::Expression;
use crate::Node;
use crate::Operator;

pub struct Engine {
    expression: Expression,
}

impl Engine {
    pub(crate) fn new(expression: Expression) -> Self {
        Self { expression }
    }

    pub fn evaluate(&self, input: &[f64]) -> Result<f64, EvaluationError> {
        if input.len() != self.expression.required_input_length {
            return Err(EvaluationError::InvalidInputLength(
                input.len(),
                self.expression.required_input_length,
            ));
        }

        const MAX_STACK_SIZE: usize = 16;
        let mut stack = tiny_vec!([f64; MAX_STACK_SIZE]);

        let expr = &self.expression.nodes[..];
        let mut operand_index = 0;
        for op in expr {
            match op {
                Node::Operand => {
                    stack.push(input[operand_index]);
                    operand_index += 1;
                }
                Node::Operator(_) => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    let result = match op {
                        Node::Operator(Operator::Add) => left + right,
                        Node::Operator(Operator::Sub) => left - right,
                        Node::Operator(Operator::Mul) => left * right,
                        Node::Operator(Operator::Div) => left / right,
                        _ => unreachable!(),
                    };
                    stack.push(result);
                }
                _ => {}
            };
        }

        assert!(stack.len() == 1);
        Ok(stack.pop().unwrap())
    }
}
