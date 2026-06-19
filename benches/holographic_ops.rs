// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Benchmarks for Minuet holographic memory operations.
//!
//! Run with: `cargo bench`
//!
//! These benchmarks measure throughput and latency for:
//! - Binding operations across algebra types
//! - Store/retrieve throughput under varying load
//! - Sharded vs. simple store scaling
//! - Resonator cleanup performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use minuet::prelude::*;

type Algebra = ProductCliffordAlgebra<64>; // 512 dimensions

/// Benchmark single store operations at various loads.
fn bench_store_at_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_load");

    for initial_load in [0, 10, 50] {
        group.bench_with_input(
            BenchmarkId::new("single_store", initial_load),
            &initial_load,
            |b, &initial_load| {
                let store = SimpleStore::<Algebra>::new();

                // Pre-load
                for _ in 0..initial_load {
                    let key = Algebra::random_versor(2);
                    let value = Algebra::random_versor(2);
                    store.store(&key, &value).unwrap();
                }

                b.iter(|| {
                    let key = Algebra::random_versor(2);
                    let value = Algebra::random_versor(2);
                    black_box(store.store(&key, &value));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark retrieval operations.
fn bench_retrieve(c: &mut Criterion) {
    let mut group = c.benchmark_group("retrieve");

    for load in [10, 50] {
        group.bench_with_input(
            BenchmarkId::new("single_retrieve", load),
            &load,
            |b, &load| {
                let store = SimpleStore::<Algebra>::new();
                let mut stored_keys = Vec::new();

                // Pre-load and track keys
                for _ in 0..load {
                    let key = Algebra::random_versor(2);
                    let value = Algebra::random_versor(2);
                    store.store(&key, &value).unwrap();
                    stored_keys.push(key);
                }

                b.iter(|| {
                    let idx = rand::random::<usize>() % stored_keys.len();
                    black_box(store.retrieve(&stored_keys[idx]));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sharded store scaling.
fn bench_sharded_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("sharded_scaling");

    for shards in [1, 4, 8] {
        group.bench_with_input(BenchmarkId::new("shards", shards), &shards, |b, &shards| {
            let store = if shards == 1 {
                ShardedStore::<Algebra>::with_shards(1)
            } else {
                ShardedStore::<Algebra>::with_shards(shards)
            };

            b.iter(|| {
                let key = Algebra::random_versor(2);
                let value = Algebra::random_versor(2);
                black_box(store.store(&key, &value));
            });
        });
    }

    group.finish();
}

/// Benchmark binding operations.
fn bench_binding(c: &mut Criterion) {
    let mut group = c.benchmark_group("binding");

    let a = Algebra::random_versor(2);
    let b = Algebra::random_versor(2);

    group.bench_function("bind", |bencher| {
        bencher.iter(|| black_box(a.bind(&b)));
    });

    group.bench_function("unbind", |bencher| {
        let bound = a.bind(&b);
        bencher.iter(|| black_box(bound.unbind(&a)));
    });

    group.bench_function("similarity", |bencher| {
        bencher.iter(|| black_box(a.similarity(&b)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_store_at_load,
    bench_retrieve,
    bench_sharded_scaling,
    bench_binding,
);
criterion_main!(benches);
