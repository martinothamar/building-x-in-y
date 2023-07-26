use std::{
    cell::RefCell,
    fs::File,
    io::{BufReader, Read},
};

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use monte_carlo_sim::{sim, TeamDto};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Simulation");

    group.throughput(Throughput::Elements(1_000));

    group.bench_function("simulation 1_000", |b| {
        let file = File::open("../input.json").unwrap();
        let mut file = BufReader::new(file);
        let mut buf = Vec::with_capacity(512);
        file.read_to_end(&mut buf).unwrap();

        let teams_dto = serde_json::from_slice::<Vec<TeamDto>>(&buf).unwrap();

        let mut state_allocator = sim::new_allocator();
        let markets_allocator = RefCell::new(sim::new_allocator());

        let state = RefCell::new(sim::State::new(&mut state_allocator, &teams_dto));

        b.iter_batched(
            || {
                state.borrow_mut().reset();
                markets_allocator.borrow_mut().reset();
            },
            |_| {
                let mut state = state.borrow_mut();
                let mut markets_allocator = markets_allocator.borrow_mut();
                let markets = sim::simulate::<1_000>(&mut state, &mut markets_allocator);
                let market = &markets[0];
                return market.outcomes[0].probability;
            },
            BatchSize::PerIteration,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
