//! Benchmarks for binding operation throughput.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use amari_fusion::{holographic::Bindable, TropicalDualClifford};
use minuet::parallel::batch::{bind_batch_parallel, bundle_parallel};

fn generate_random_pairs<const DIM: usize>(
    n: usize,
) -> Vec<(
    TropicalDualClifford<f64, DIM>,
    TropicalDualClifford<f64, DIM>,
)> {
    (0..n)
        .map(|_| {
            (
                TropicalDualClifford::random(),
                TropicalDualClifford::random(),
            )
        })
        .collect()
}

fn binding_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("binding");

    for size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));

        // Sequential binding - 64 dimensions
        group.bench_with_input(
            BenchmarkId::new("sequential/64", size),
            &size,
            |b, &size| {
                let pairs = generate_random_pairs::<8>(size);
                b.iter(|| {
                    for (k, v) in &pairs {
                        black_box(k.bind(v));
                    }
                });
            },
        );

        // Parallel binding - 64 dimensions
        group.bench_with_input(BenchmarkId::new("parallel/64", size), &size, |b, &size| {
            let pairs = generate_random_pairs::<8>(size);
            let (keys, values): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
            b.iter(|| {
                black_box(bind_batch_parallel(&keys, &values));
            });
        });

        // Sequential binding - 256 dimensions
        group.bench_with_input(
            BenchmarkId::new("sequential/256", size),
            &size,
            |b, &size| {
                let pairs = generate_random_pairs::<8>(size);
                b.iter(|| {
                    for (k, v) in &pairs {
                        black_box(k.bind(v));
                    }
                });
            },
        );

        // Parallel binding - 256 dimensions
        group.bench_with_input(BenchmarkId::new("parallel/256", size), &size, |b, &size| {
            let pairs = generate_random_pairs::<8>(size);
            let (keys, values): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
            b.iter(|| {
                black_box(bind_batch_parallel(&keys, &values));
            });
        });
    }

    group.finish();
}

fn bundling_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundling");

    for size in [10, 100, 1000] {
        group.throughput(Throughput::Elements(size as u64));

        // Sequential bundling
        group.bench_with_input(
            BenchmarkId::new("sequential/64", size),
            &size,
            |b, &size| {
                let items: Vec<TropicalDualClifford<f64, 8>> =
                    (0..size).map(|_| TropicalDualClifford::random()).collect();
                b.iter(|| {
                    let mut result = TropicalDualClifford::bundling_zero();
                    for item in &items {
                        result = result.bundle(item, 1.0);
                    }
                    black_box(result)
                });
            },
        );

        // Parallel bundling
        group.bench_with_input(BenchmarkId::new("parallel/64", size), &size, |b, &size| {
            let items: Vec<TropicalDualClifford<f64, 8>> =
                (0..size).map(|_| TropicalDualClifford::random()).collect();
            b.iter(|| {
                black_box(bundle_parallel(&items, 1.0));
            });
        });
    }

    group.finish();
}

fn dimension_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimension_scaling");

    // Single bind at various dimensions
    group.bench_function("bind/32", |b| {
        let a: TropicalDualClifford<f64, 32> = TropicalDualClifford::random();
        let c: TropicalDualClifford<f64, 32> = TropicalDualClifford::random();
        b.iter(|| black_box(a.bind(&c)));
    });

    group.bench_function("bind/64", |b| {
        let a: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        let c: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        b.iter(|| black_box(a.bind(&c)));
    });

    group.bench_function("bind/128", |b| {
        let a: TropicalDualClifford<f64, 16> = TropicalDualClifford::random();
        let c: TropicalDualClifford<f64, 16> = TropicalDualClifford::random();
        b.iter(|| black_box(a.bind(&c)));
    });

    group.bench_function("bind/256", |b| {
        let a: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        let c: TropicalDualClifford<f64, 8> = TropicalDualClifford::random();
        b.iter(|| black_box(a.bind(&c)));
    });

    group.finish();
}

criterion_group!(
    benches,
    binding_throughput,
    bundling_throughput,
    dimension_scaling
);
criterion_main!(benches);
