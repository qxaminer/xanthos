use criterion::{criterion_group, criterion_main, Criterion};

fn inference_benchmark(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| {
            // TODO: Actual inference benchmark
            std::hint::black_box(42)
        })
    });
}

criterion_group!(benches, inference_benchmark);
criterion_main!(benches);
