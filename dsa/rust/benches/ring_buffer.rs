use std::collections::VecDeque;

use criterion::{criterion_group, criterion_main, Criterion};
use dsa::ring_buffer::RingBuffer;

fn do_rb<const N: usize>(rb: &mut RingBuffer<usize, N>) -> usize {
    for i in 1..N {
        *rb.push().unwrap() = i;
    }

    let mut v: usize = 0;
    for _ in 1..N {
        v += *rb.pop().unwrap();
    }

    v
}

fn do_vd<const N: usize>(vd: &mut VecDeque<usize>) -> usize {
    for i in 1..N {
        vd.push_back(i);
    }

    let mut v: usize = 0;
    for _ in 1..N {
        v += vd.pop_front().unwrap();
    }

    v
}

fn criterion_benchmark(c: &mut Criterion) {
    const N: usize = 64;

    let mut group = c.benchmark_group("RingBuffers");

    group.bench_function("dsa::ring_buffer::RingBuffer heap 64", |b| {
        let mut rb = RingBuffer::<usize, N>::new_heap();

        b.iter(|| do_rb::<N>(&mut rb))
    });

    group.bench_function("dsa::ring_buffer::RingBuffer inline 64", |b| {
        let mut rb = RingBuffer::<usize, N>::new_inline();

        b.iter(|| do_rb::<N>(&mut rb))
    });

    group.bench_function("std::collections::VecDeque 64", |b| {
        let mut vd = VecDeque::<usize>::with_capacity(N);

        b.iter(|| do_vd::<N>(&mut vd))
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
