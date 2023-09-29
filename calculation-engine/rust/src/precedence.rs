use std::sync::LazyLock;

use crate::Node;

struct Precedence {
    min: i32,
    values: Vec<i32>,
}
static PRECEDENCE: LazyLock<Precedence> = LazyLock::new(|| {
    #[derive(Clone)]
    struct Op(char, i32);

    let ops = [Op('+', 1), Op('-', 1), Op('*', 2), Op('/', 2), Op('^', 3)];
    let min = ops.iter().map(|v| v.0 as i32).min().unwrap();
    let max = ops.iter().map(|v| v.0 as i32).max().unwrap();

    let mut precedence = vec![0; (max - min + 1) as usize];

    for i in 0..(max - min + 1) {
        let mut op: Option<Op> = None;

        for op_match in &ops {
            if (op_match.0 as i32) - min == i {
                op = Some(op_match.clone());
                break;
            }
        }

        if let Some(Op(_, p)) = op {
            precedence[i as usize] = p;
        }
    }

    Precedence {
        min,
        values: precedence,
    }
});

pub(crate) fn precedence(n: Node) -> i32 {
    match n {
        Node::Operator(value) => PRECEDENCE.values[((value as i32) - PRECEDENCE.min) as usize],
        _ => -1i32,
    }
}
