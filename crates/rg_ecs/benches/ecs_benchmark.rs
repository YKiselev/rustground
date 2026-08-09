use criterion::{criterion_group, criterion_main, Criterion};
use rg_ecs::{
    archetype::{build_archetype, ArchetypeId},
    entity::{Entities, EntityId},
};
use std::hint::black_box;

#[derive(Default, Debug)]
struct Location(f32, f32, f32);
#[derive(Default, Debug)]
struct Velocity(f32, f32, f32);
#[derive(Default, Debug)]
struct Direction(f32, f32, f32);
#[derive(Default, Debug)]
struct Name(String);

fn init_storage(chunk_size: usize, count: Option<usize>) -> (Entities, ArchetypeId, ArchetypeId) {
    let entities = Entities::new(chunk_size);
    let arch_id1 = entities.add_archetype(build_archetype! {i32, f64, String});
    let arch_id2 = entities
        .add_archetype(build_archetype! {Location, Velocity, Direction, Name, bool, char, i8, i16});
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
    let mut counter = 1usize;
    entities.visit(
        |l: &mut Location,
         v: &mut Velocity,
         d: &mut Direction,
         n: &mut Name,
         flag: &mut bool,
         ch: &mut char,
         i1: &mut i8,
         i2: &mut i16| {
            l.0 = (counter + 1) as f32;
            l.1 = (counter + 2) as f32;
            l.2 = (counter + 3) as f32;
            counter += 3;
            v.0 = (counter + 1) as f32;
            v.1 = (counter + 2) as f32;
            v.2 = (counter + 3) as f32;
            counter += 3;
            d.0 = (counter + 1) as f32;
            d.1 = (counter + 2) as f32;
            d.2 = (counter + 3) as f32;
            counter += 3;
            n.0 = format!("Test_{}", counter);
            *flag = if counter & 1 != 0 { true } else { false };
            *ch = counter as u8 as char;
            *i1 = counter as i8;
            *i2 = counter as i16;
        },
    );
    (entities, arch_id1, arch_id2)
}

fn ecs_benchmark(c: &mut Criterion) {
    /*
    let (entities, arch_id1, arch_id2) = init_storage(128 * 1024, None);

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
    });*/
    let fn_1 = |vel: &Location, dir: &Direction| {
        black_box(vel);
        black_box(dir);
    };
    let fn_2 =
        |v1: &EntityId, loc: &mut Location, vel: &Velocity, dir: &Direction| {
            black_box(v1);
            loc.0 += dir.0 * vel.0;
            loc.1 += dir.1 * vel.1;
            loc.2 += dir.2 * vel.2;
            black_box(loc);
            black_box(vel);
            black_box(dir);
        };
    let fn_5 =
        |id: &EntityId, loc: &mut Location, vel: &Velocity, dir: &Direction| {
            black_box(id);
            //black_box(tag);
            loc.0 += dir.0 * vel.0;
            loc.1 += dir.1 * vel.1;
            loc.2 += dir.2 * vel.2;
            black_box(loc);
            black_box(dir);
            black_box(vel);
        };
    for chunk_size in [128, 512, 1024] {
        let chunk_size = chunk_size * 1024;
        let (entities, _, _) = init_storage(chunk_size, Some(black_box(1_000_000)));
        // let name = format!("ecs visit 1-arg system (chunk_size={chunk_size})");
        // c.bench_function(&name, |b| {
        //     b.iter(|| entities.visit(fn_1));
        // });
        // let name = format!("ecs visit 2-arg system (chunk_size={chunk_size})");
        // c.bench_function(&name, |b| {
        //     b.iter(|| assert!(entities.visit(fn_2).2 > 0));
        // });
        let name = format!("ecs visit 5-arg system (chunk_size={chunk_size})");
        c.bench_function(&name, |b| {
            b.iter(|| assert!(entities.visit(fn_5).2 > 0));
        });
        black_box(entities);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    ecs_benchmark(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
