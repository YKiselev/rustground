use criterion::{criterion_group, criterion_main, Criterion};
use itertools::Itertools;
use rg_ecs::{
    archetype::build_archetype,
    entity::{Entities, EntityId},
    visitor::{Tuple1, Tuple2},
};
use std::{hint::black_box, sync::atomic::AtomicI64};

#[derive(Default)]
struct Location(f32, f32, f32);
#[derive(Default)]
struct Velocity(f32, f32, f32);
#[derive(Default)]
struct Name(String);

fn ecs_benchmark(c: &mut Criterion) {
    let entities = Entities::new();
    let arch_id1 = entities.add_archetype(build_archetype! {i32, f64, String});
    let arch_id2 =
        entities.add_archetype(build_archetype! {Location, Velocity, Name, bool, char, i8, i16});

    c.bench_function("ecs add arch #1", |b| {
        b.iter(|| entities.add(Some(black_box(arch_id1))))
    });
    c.bench_function("ecs add arch #2", |b| {
        b.iter(|| entities.add(Some(black_box(arch_id2))))
    });
    c.bench_function("ecs move 1000", |b| {
        b.iter_batched(
            || {
                (0..1000)
                    .map(|i| entities.add(Some(arch_id1)).unwrap())
                    .collect::<Vec<_>>()
            },
            |batch| {
                batch
                    .iter()
                    .map(|ent_id| entities.set(*ent_id, black_box(Velocity(1.0, 2.0, 3.0))))
                    .count()
            },
            criterion::BatchSize::SmallInput,
        )
    });
    c.bench_function("ecs remove 1000", |b| {
        b.iter_batched(
            || {
                (0..1000)
                    .map(|i| entities.add(Some(arch_id1)).unwrap())
                    .collect::<Vec<_>>()
            },
            |batch| batch.iter().map(|ent_id| entities.remove(*ent_id)).count(),
            criterion::BatchSize::SmallInput,
        )
    });
    c.bench_function("ecs visit 1000", |b| {
        b.iter_batched(
            || {
                (0..1000)
                    .map(|i| entities.add(Some(arch_id1)).unwrap())
                    .collect::<Vec<_>>()
            },
            |_| {
                let tup2 = Tuple2::<EntityId, String>::new(|(v1, v2)| {
                    black_box(v1);
                    black_box(v2);
                });
                entities.visit(&tup2);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    ecs_benchmark(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
