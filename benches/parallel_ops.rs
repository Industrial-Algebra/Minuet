//! Benchmarks for parallel operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use amari_fusion::TropicalDualClifford;
use minuet::memory::MemoryStore;
use minuet::parallel::batch::{
    bind_batch_parallel, normalize_batch_parallel, similarities_parallel, top_k_parallel,
};
use minuet::parallel::ShardedMemory;

fn parallel_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_similarity");

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("full_scan/64", size), &size, |b, &size| {
            let query: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
            let candidates: Vec<TropicalDualClifford<f64, 64>> =
                (0..size).map(|_| TropicalDualClifford::random()).collect();

            b.iter(|| {
                black_box(similarities_parallel(&query, &candidates));
            });
        });

        group.bench_with_input(BenchmarkId::new("top_10/64", size), &size, |b, &size| {
            let query: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
            let candidates: Vec<TropicalDualClifford<f64, 64>> =
                (0..size).map(|_| TropicalDualClifford::random()).collect();

            b.iter(|| {
                black_box(top_k_parallel(&query, &candidates, 10));
            });
        });
    }

    group.finish();
}

fn parallel_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_normalize");

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("batch/64", size), &size, |b, &size| {
            let items: Vec<TropicalDualClifford<f64, 64>> =
                (0..size).map(|_| TropicalDualClifford::random()).collect();

            b.iter(|| {
                black_box(normalize_batch_parallel(&items));
            });
        });
    }

    group.finish();
}

fn sharded_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("sharded_memory");

    // Store throughput
    group.bench_function("store_batch/4_shards", |b| {
        let memory: ShardedMemory<f64, 64, 4> = ShardedMemory::new();
        let pairs: Vec<_> = (0..100)
            .map(|_| {
                (
                    TropicalDualClifford::random(),
                    TropicalDualClifford::random(),
                )
            })
            .collect();

        b.iter(|| {
            black_box(memory.store_batch(&pairs).unwrap());
        });
    });

    // Retrieval with pre-loaded data
    group.bench_function("retrieve/4_shards/100_items", |b| {
        let memory: ShardedMemory<f64, 64, 4> = ShardedMemory::new();

        let mut keys = Vec::new();
        for _ in 0..100 {
            let key: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
            let value: TropicalDualClifford<f64, 64> = TropicalDualClifford::random();
            memory.store(&key, &value).unwrap();
            keys.push(key);
        }

        let query_key = &keys[0];
        b.iter(|| {
            black_box(memory.retrieve(query_key).unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    parallel_similarity,
    parallel_normalization,
    sharded_memory
);
criterion_main!(benches);
