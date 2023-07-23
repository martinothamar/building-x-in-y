use std::{fs::File, io::{BufReader, Read}, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use monte_carlo_sim::{TeamDto, sim};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Simulation");

    group.throughput(Throughput::Elements(100_000));
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("simulation 100_000", |b| {
        let file = File::open("../input.json").unwrap();
        let mut file = BufReader::new(file);
        let mut buf = Vec::with_capacity(512);
        file.read_to_end(&mut buf).unwrap();

        let teams_dto = serde_json::from_slice::<Vec<TeamDto>>(&buf).unwrap();

        let mut state = sim::State::new(&teams_dto);

        b.iter(|| sim::simulate::<100_000>(black_box(&mut state)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
