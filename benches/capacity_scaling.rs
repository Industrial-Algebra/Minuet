//! Benchmarks for capacity scaling behavior.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use minuet::memory::{BasicMemoryStore, MemoryStore};

fn store_at_various_loads(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_load");

    for initial_load in [0, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("single_store/64", initial_load),
            &initial_load,
            |b, &initial_load| {
                let store: BasicMemoryStore<f64, 64> = BasicMemoryStore::new();

                // Pre-load
                for _ in 0..initial_load {
                    let key: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                    let value: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                    store.store(&key, &value).unwrap();
                }

                // Benchmark single store
                b.iter(|| {
                    let key: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                    let value: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                    black_box(store.store(&key, &value));
                });
            },
        );
    }

    group.finish();
}

fn capacity_info_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("capacity_info");

    for load in [10, 100, 500] {
        group.bench_with_input(BenchmarkId::new("compute", load), &load, |b, &load| {
            let store: BasicMemoryStore<f64, 64> = BasicMemoryStore::new();

            for _ in 0..load {
                let key: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                let value: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                store.store(&key, &value).unwrap();
            }

            b.iter(|| {
                black_box(store.capacity());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, store_at_various_loads, capacity_info_computation);
criterion_main!(benches);
