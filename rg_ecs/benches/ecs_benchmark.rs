use criterion::{criterion_group, criterion_main, Criterion};
use itertools::Itertools;
use rg_ecs::{
    archetype::build_archetype,
    component::ComponentId,
    entity::{Entities, EntityId},
    visitor::visit_2,
};
use std::{
    collections::HashSet,
    hint::black_box,
    sync::atomic::{AtomicI64, AtomicUsize},
};

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
                    .map(|_| entities.add(Some(arch_id1)).unwrap())
                    .collect::<Vec<_>>()
            },
            |batch| batch.iter().map(|ent_id| entities.remove(*ent_id)).count(),
            criterion::BatchSize::SmallInput,
        )
    });
    entities.clear();
    let c1 = (0..1000000)
        .map(|_| entities.add(Some(arch_id1)).unwrap())
        .count();
    let c2 = (0..1000000)
        .map(|_| entities.add(Some(arch_id2)).unwrap())
        .count();
    {
        let columns = HashSet::from([ComponentId::new::<EntityId>(), ComponentId::new::<String>()]);
        c.bench_function("ecs visit", |b| {
            b.iter(|| {
                entities.visit(
                    &columns,
                    visit_2(|(v1, v2): (&EntityId, &String)| {
                        black_box(v1);
                        black_box(v2);
                    }),
                )
            });
        });
    }
    {
        let columns = HashSet::from([ComponentId::new::<Velocity>(), ComponentId::new::<Name>()]);
        c.bench_function("ecs visit e2", |b| {
            b.iter(|| {
                entities.visit(
                    &columns,
                    visit_2(|(v1, v2): (&Velocity, &Name)| {
                        black_box(v1);
                        black_box(v2);
                    }),
                )
            });
        });
    }
    println!("Storage has {} e1 and {} e2 entities.", c1, c2);
}

fn criterion_benchmark(c: &mut Criterion) {
    ecs_benchmark(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
