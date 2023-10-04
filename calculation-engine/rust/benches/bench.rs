#![feature(iter_repeat_n)]

use std::iter::repeat_n;

use calculation_engine::*;
use criterion::{criterion_group, criterion_main, Criterion};

fn get_simple_expression_nodes() -> Expression {
    let expression = vec![
        Node::Operand,
        Node::Operator(Operator::Add),
        Node::LeftParens,
        Node::Operand,
        Node::Operator(Operator::Sub),
        Node::Operand,
        Node::RightParens,
    ];
    Expression::from_infix(&expression).unwrap()
}

const SIZE: usize = 1024 * 8;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("scalar {}", SIZE), |b| {
        let expression = get_simple_expression_nodes();
        let engine = expression.to_scalar_engine();
        let mut results = vec![0f64; SIZE];
        let input = vec![1.0, 2.0, 1.0];
        b.iter(|| {
            for el in &mut results {
                *el = engine.evaluate(&input).unwrap();
            }
        })
    });

    c.bench_function(&format!("vectorized {}", SIZE), |b| {
        let expression = get_simple_expression_nodes();
        let engine = expression.to_vectorized_engine();
        let mut results = vec![0f64; SIZE];
        let input = [
            repeat_n(1.0, SIZE).collect::<Vec<_>>(),
            repeat_n(2.0, SIZE).collect::<Vec<_>>(),
            repeat_n(1.0, SIZE).collect::<Vec<_>>(),
        ];
        let input = input.iter().map(|v| &v[..]).collect::<Vec<_>>();
        b.iter(|| {
            engine.evaluate(&input, &mut results).unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
