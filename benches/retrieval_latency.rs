//! Benchmarks for retrieval latency.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use minuet::memory::{BasicMemoryStore, MemoryStore};

fn retrieval_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("retrieval");

    for load in [10, 50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::new("single_key/64", load),
            &load,
            |b, &load| {
                let store: BasicMemoryStore<f64, 64> = BasicMemoryStore::new();

                // Pre-load store
                let mut keys = Vec::with_capacity(load);
                for _ in 0..load {
                    let key: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                    let value: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                    store.store(&key, &value).unwrap();
                    keys.push(key);
                }

                // Benchmark retrieval of first key
                let query_key = &keys[0];
                b.iter(|| {
                    black_box(store.retrieve(query_key).unwrap());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("single_key/256", load),
            &load,
            |b, &load| {
                let store: BasicMemoryStore<f64, 256> = BasicMemoryStore::new();

                let mut keys = Vec::with_capacity(load);
                for _ in 0..load {
                    let key: TropicalDualClifford<f64, 256> = TropicalDualClifford::random();
                    let value: TropicalDualClifford<f64, 256> = TropicalDualClifford::random();
                    store.store(&key, &value).unwrap();
                    keys.push(key);
                }

                let query_key = &keys[0];
                b.iter(|| {
                    black_box(store.retrieve(query_key).unwrap());
                });
            },
        );
    }

    group.finish();
}

fn similarity_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity");

    for dim in [64, 128, 256] {
        match dim {
            64 => {
                let a: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                let b: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
                group.bench_function(BenchmarkId::new("pair", dim), |bench| {
                    bench.iter(|| black_box(a.similarity(&b)));
                });
            }
            128 => {
                let a: TropicalDualClifford<f64, 128> = TropicalDualClifford::random();
                let b: TropicalDualClifford<f64, 128> = TropicalDualClifford::random();
                group.bench_function(BenchmarkId::new("pair", dim), |bench| {
                    bench.iter(|| black_box(a.similarity(&b)));
                });
            }
            256 => {
                let a: TropicalDualClifford<f64, 256> = TropicalDualClifford::random();
                let b: TropicalDualClifford<f64, 256> = TropicalDualClifford::random();
                group.bench_function(BenchmarkId::new("pair", dim), |bench| {
                    bench.iter(|| black_box(a.similarity(&b)));
                });
            }
            _ => {}
        }
    }

    group.finish();
}

criterion_group!(benches, retrieval_latency, similarity_computation);
criterion_main!(benches);
