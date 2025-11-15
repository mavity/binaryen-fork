use criterion::{black_box, Criterion, criterion_group, criterion_main};
use binaryen_support::StringInterner;
use binaryen_support::Arena;

fn bench_intern(c: &mut Criterion) {
    let interner = StringInterner::new();
    c.bench_function("intern_hello", |b| {
        b.iter(|| {
            let s = interner.intern(black_box("hello"));
            black_box(s);
        })
    });
}

fn bench_arena_alloc(c: &mut Criterion) {
    let a = Arena::new();
    c.bench_function("alloc_str", |b| {
        b.iter(|| {
            let p = a.alloc_str(black_box("arena-hello"));
            black_box(p);
        })
    });
}

criterion_group!(benches, bench_intern, bench_arena_alloc);
criterion_main!(benches);
