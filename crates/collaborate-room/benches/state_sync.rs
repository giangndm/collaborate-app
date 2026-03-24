use collaborate_room::State;
use criterion::{
    BenchmarkId, Criterion, SamplingMode, Throughput, black_box, criterion_group, criterion_main,
};
use std::time::Duration;
use syncable_state::{
    PathSegment, SyncError, SyncPath, SyncableCounter, SyncableState, SyncableString,
};

#[derive(Debug, Clone, SyncableState)]
struct BenchState {
    #[sync(id)]
    pub id: String,
    pub label: SyncableString,
    pub v: SyncableCounter,
}


impl Default for BenchState {
    fn default() -> Self {
        let root = SyncPath::from_field("bench");
        let mut path_label = root.clone().into_vec();
        path_label.push(PathSegment::Field("label".into()));

        let mut path_v = root.clone().into_vec();
        path_v.push(PathSegment::Field("v".into()));

        Self {
            id: "bench".into(),
            label: SyncableString::new(SyncPath::new(path_label), "bench"),
            v: SyncableCounter::new(SyncPath::new(path_v), 0),
        }
    }
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
                    let mut sender = State::with_node_id("sender", BenchState::default());
                    let mut receiver = State::with_node_id("receiver", BenchState::default());

                    for _ in 0..black_box(changes) {
                        sender
                            .mutate(|state, batch| {
                                state.v.increment(batch, 1)?;
                                Ok::<(), SyncError>(())
                            })
                            .unwrap();
                        sync_all(&mut sender, &mut receiver);
                    }

                    assert_eq!(sender.v.value(), receiver.v.value());
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
                let mut state = State::with_node_id("idle", BenchState::default());

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
