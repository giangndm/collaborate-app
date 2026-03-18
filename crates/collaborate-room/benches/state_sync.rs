use automorph::Automorph;
use collaborate_room::State;
use criterion::{
    BenchmarkId, Criterion, SamplingMode, Throughput, black_box, criterion_group, criterion_main,
};
use std::time::Duration;

#[derive(Debug, Default, Automorph, PartialEq, Eq)]
struct BenchState {
    label: String,
    v: u32,
}

fn sync_all(sender: &mut State<BenchState>, receiver: &mut State<BenchState>) {
    while let Some(change) = sender.poll() {
        receiver.apply(change);
    }
}

fn bench_single_field_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_field_sync");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(10));

    for changes in [100_u32, 300, 1_000] {
        group.throughput(Throughput::Elements(u64::from(changes)));
        group.bench_with_input(
            BenchmarkId::from_parameter(changes),
            &changes,
            |b, &changes| {
                b.iter(|| {
                    let mut sender = State::with_node_id(
                        "sender",
                        BenchState {
                            label: "bench".to_string(),
                            ..BenchState::default()
                        },
                    );
                    let mut receiver = State::with_node_id(
                        "receiver",
                        BenchState {
                            label: "bench".to_string(),
                            ..BenchState::default()
                        },
                    );

                    for _ in 0..black_box(changes) {
                        sender.v += 1;
                        sync_all(&mut sender, &mut receiver);
                    }

                    assert_eq!(&*sender, &*receiver);
                });
            },
        );
    }

    group.finish();
}

fn bench_idle_poll(c: &mut Criterion) {
    let mut group = c.benchmark_group("idle_poll");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(10));

    for polls in [1_000_u64, 10_000, 100_000] {
        group.throughput(Throughput::Elements(polls));
        group.bench_with_input(BenchmarkId::from_parameter(polls), &polls, |b, &polls| {
            b.iter(|| {
                let mut state = State::with_node_id(
                    "idle",
                    BenchState {
                        label: "idle".to_string(),
                        ..BenchState::default()
                    },
                );

                while state.poll().is_some() {}

                for _ in 0..black_box(polls) {
                    assert!(state.poll().is_none());
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_single_field_sync, bench_idle_poll);
criterion_main!(benches);
