use criterion::{criterion_group, criterion_main, Criterion};
use rg_ecs::{
    archetype::{build_archetype, ArchetypeId},
    component::ComponentId,
    entity::{Entities, EntityId},
    visitor::visit_2,
};
use std::{collections::HashSet, hint::black_box};

#[derive(Default)]
struct Location(f32, f32, f32);
#[derive(Default)]
struct Velocity(f32, f32, f32);
#[derive(Default)]
struct Name(String);

fn init_storage(chunk_size: usize, count: Option<usize>) -> (Entities, ArchetypeId, ArchetypeId) {
    let entities = Entities::new(chunk_size);
    let arch_id1 = entities.add_archetype(build_archetype! {i32, f64, String});
    let arch_id2 =
        entities.add_archetype(build_archetype! {Location, Velocity, Name, bool, char, i8, i16});
    if let Some(count) = count {
        let c1 = (0..count)
            .map(|_| entities.add(Some(arch_id1)).unwrap())
            .count();
        let c2 = (0..count)
            .map(|_| entities.add(Some(arch_id2)).unwrap())
            .count();
        black_box(c1);
        black_box(c2);
    }
    (entities, arch_id1, arch_id2)
}

fn ecs_benchmark(c: &mut Criterion) {
    let (entities, arch_id1, arch_id2) = init_storage(2048, None);

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
    let columns1 = HashSet::from([ComponentId::new::<EntityId>(), ComponentId::new::<String>()]);
    let columns2 = HashSet::from([ComponentId::new::<Velocity>(), ComponentId::new::<Name>()]);
    for chunk_size in [16 * 1024, 32 * 1024, 64 * 1024, 128 * 1024] {
        let (entities, _, _) = init_storage(chunk_size, Some(1000000));
        let name = format!("ecs visit e1 (chunk_size={chunk_size})");
        c.bench_function(&name, |b| {
            b.iter(|| {
                entities.visit(
                    &columns1,
                    visit_2(|(v1, v2): (&EntityId, &String)| {
                        black_box(v1);
                        black_box(v2);
                    }),
                )
            });
        });
        let name = format!("ecs visit e2 (chunk_size={chunk_size})");
        c.bench_function(&name, |b| {
            b.iter(|| {
                entities.visit(
                    &columns2,
                    visit_2(|(v1, v2): (&Velocity, &Name)| {
                        black_box(v1);
                        black_box(v2);
                    }),
                )
            });
        });
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    ecs_benchmark(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
