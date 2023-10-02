struct MinStack {
    data: Vec<Item>,
}

#[derive(Debug, Clone, Copy)]
struct Item(i32, i32);

impl MinStack {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn push(&mut self, val: i32) {
        let min = match self.data.last() {
            Some(&Item(_, m)) => m.min(val),
            None => val,
        };
        self.data.push(Item(val, min));
    }

    fn pop(&mut self) {
        self.data.pop().unwrap();
    }

    fn top(&self) -> i32 {
        self.data.last().unwrap().0
    }

    fn get_min(&self) -> i32 {
        self.data.last().unwrap().1
    }
}
