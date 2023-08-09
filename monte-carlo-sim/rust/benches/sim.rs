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

        // Using som RefCell's here, since I cant mutably borrow these in both of the
        // iter_batched closures. Do dynamic borrow checking instead.
        // Perf hit is negligible, since one iteration runs in the millisecond range
        let state_allocator = sim::new_allocator();
        let markets_allocator = RefCell::new(sim::new_allocator());

        let state = RefCell::new(sim::State::new(&state_allocator, &teams_dto));

        b.iter_batched(
            || {
                state.borrow_mut().reset();
                markets_allocator.borrow_mut().reset();
            },
            |_| {
                let mut state = state.borrow_mut();
                let markets_allocator = markets_allocator.borrow_mut();
                // Stick to 1'000 simulations, as that keeps the running time to within a couple of milliseconds.
                // hopefully then the iterations are within a single OS scheduler timeslice
                let markets = sim::simulate::<1_000>(&mut state, &markets_allocator);
                let market = &markets[0];
                market.outcomes[0].probability
            },
            BatchSize::PerIteration,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
