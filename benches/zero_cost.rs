use criterion::{black_box, criterion_group, criterion_main, Criterion};
use faultkit::{clear, should_fail_mmap};

fn bench_zero_cost(c: &mut Criterion) {
    let _ = clear(); // ensure disabled
    c.bench_function("should_fail_mmap_disabled", |b| {
        b.iter(|| black_box(should_fail_mmap()));
    });
}

criterion_group!(benches, bench_zero_cost);
criterion_main!(benches);
