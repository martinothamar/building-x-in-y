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

        let state = RefCell::new(sim::State::new(&teams_dto));

        b.iter_batched(
            || {
                state.borrow_mut().reset();
            },
            |_| sim::simulate::<1_000>(&mut *state.borrow_mut()),
            BatchSize::PerIteration,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
