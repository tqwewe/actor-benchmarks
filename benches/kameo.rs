use std::{sync::Arc, time::Instant};

use criterion::{criterion_group, criterion_main, Criterion};
use kameo::{
    message::{Context, Message},
    request::MessageSend,
    Actor,
};
use tokio::{
    runtime::Runtime,
    sync::{OwnedSemaphorePermit, Semaphore},
};

#[derive(Actor)]
#[actor(mailbox = unbounded)]
struct UnboundedCounter {
    count: i64,
}

#[derive(Actor)]
#[actor(mailbox = bounded(1_000_000))]
struct BoundedCounter {
    count: i64,
}

struct Inc {
    amount: i64,
    #[allow(dead_code)]
    permit: Option<OwnedSemaphorePermit>,
}

impl Message<Inc> for UnboundedCounter {
    type Reply = i64;

    async fn handle(&mut self, msg: Inc, _ctx: Context<'_, Self, Self::Reply>) -> Self::Reply {
        self.count += msg.amount;
        self.count
    }
}

impl Message<Inc> for BoundedCounter {
    type Reply = i64;

    async fn handle(&mut self, msg: Inc, _ctx: Context<'_, Self, Self::Reply>) -> Self::Reply {
        self.count += msg.amount;
        self.count
    }
}

fn benchmark_tell_unbounded(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let num_actors = 100;

    c.bench_function("Kameo Tell Unbounded", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut actor_refs = Vec::with_capacity(num_actors);
            for _ in 0..num_actors {
                let actor_ref = kameo::spawn(UnboundedCounter { count: 0 });
                actor_ref
                    .ask(Inc {
                        amount: 0,
                        permit: None,
                    })
                    .await
                    .unwrap();
                actor_refs.push(actor_ref);
            }

            let semaphore = Arc::new(Semaphore::new(iters.try_into().unwrap()));
            let permits = (0..iters).map(|_| semaphore.clone().try_acquire_owned().unwrap());

            let start = Instant::now();

            for (i, permit) in permits.into_iter().enumerate() {
                let actor_ref = &actor_refs[i % num_actors];
                actor_ref
                    .tell(Inc {
                        amount: 1,
                        permit: Some(permit),
                    })
                    .send()
                    .await
                    .unwrap();
            }

            let _ = semaphore
                .acquire_many(iters.try_into().unwrap())
                .await
                .unwrap();

            start.elapsed()
        })
    });
}

fn benchmark_tell_bounded(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let num_actors = 100;

    c.bench_function("Kameo Tell Bounded", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut actor_refs = Vec::with_capacity(num_actors);
            for _ in 0..num_actors {
                let actor_ref = kameo::spawn(BoundedCounter { count: 0 });
                actor_refs.push(actor_ref);
            }

            let semaphore = Arc::new(Semaphore::new(iters.try_into().unwrap()));
            let permits = (0..iters).map(|_| semaphore.clone().try_acquire_owned().unwrap());

            let start = Instant::now();

            for (i, permit) in permits.into_iter().enumerate() {
                let actor_ref = &actor_refs[i % num_actors];
                actor_ref
                    .tell(Inc {
                        amount: 1,
                        permit: Some(permit),
                    })
                    .send()
                    .await
                    .unwrap();
            }

            let _ = semaphore
                .acquire_many(iters.try_into().unwrap())
                .await
                .unwrap();

            start.elapsed()
        })
    });
}

fn benchmark_actor_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("Kameo Actor Creation", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let start = Instant::now();

            for _ in 0..iters {
                let _ = kameo::spawn(UnboundedCounter { count: 0 });
            }

            start.elapsed()
        })
    });
}

criterion_group!(
    kameo,
    benchmark_tell_unbounded,
    benchmark_tell_bounded,
    benchmark_actor_creation,
);
criterion_main!(kameo);
