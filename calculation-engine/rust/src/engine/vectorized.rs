use std::{cmp::Ordering, slice};

use tinyvec::tiny_vec;

use crate::{Expression, Node, Operator};

use super::EvaluationError;

pub struct Engine {
    expression: Expression,
}

impl Engine {
    pub(crate) fn new(expression: Expression) -> Self {
        Self { expression }
    }

    pub fn evaluate(&self, input: &[&[f64]], output: &mut [f64]) -> Result<(), EvaluationError> {
        if input.len() != self.expression.required_input_length {
            return Err(EvaluationError::InvalidInputLength(
                input.len(),
                self.expression.required_input_length,
            ));
        }

        let expected_count = input[0].len();
        for (i, &column) in input[1..].iter().enumerate() {
            if column.len() != expected_count {
                return Err(EvaluationError::InvalidInputColumnLength(
                    column.len(),
                    expected_count,
                    i,
                ));
            }
        }

        use std::simd::f64x4 as vector;

        let lanes = vector::LANES;

        const MAX_STACK_SIZE: usize = 16;
        let mut stack = tiny_vec!([usize; MAX_STACK_SIZE]);

        let expr = &self.expression.nodes[..];
        let mut operand_index = 0;
        for op in expr {
            match op {
                Node::Operand => {
                    stack.push(operand_index);
                    operand_index += 1;
                }
                Node::Operator(operator) => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();

                    let mut out = output.as_mut_ptr();

                    let left_col = match left.cmp(&input.len()) {
                        Ordering::Less => input[left],
                        _ => output,
                    };
                    let right_col = match right.cmp(&input.len()) {
                        Ordering::Less => input[right],
                        _ => output,
                    };

                    assert!(left_col.len() == right_col.len());

                    let mut j = 0;
                    while j < expected_count && expected_count - j >= lanes {
                        let l = vector::from_slice(&left_col[j..j + 4]);
                        let r = vector::from_slice(&right_col[j..j + 4]);

                        let result = match operator {
                            Operator::Add => l + r,
                            Operator::Sub => l - r,
                            Operator::Mul => l * r,
                            Operator::Div => l / r,
                        };

                        result.copy_to_slice(unsafe { slice::from_raw_parts_mut(out, lanes) });
                        out = unsafe { out.add(lanes) };

                        j += lanes;
                    }

                    while j < expected_count {
                        let l = left_col[j];
                        let r = right_col[j];

                        let result = match operator {
                            Operator::Add => l + r,
                            Operator::Sub => l - r,
                            Operator::Mul => l * r,
                            Operator::Div => l / r,
                        };

                        unsafe {
                            *out = result;
                            out = out.add(1);
                        }

                        j += 1;
                    }

                    stack.push(input.len());
                }
                Node::LeftParens => todo!(),
                Node::RightParens => todo!(),
            }
        }

        assert!(stack.len() == 1);

        Ok(())
    }
}
