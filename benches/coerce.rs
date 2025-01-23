use std::{sync::Arc, time::Instant};

use coerce::actor::{
    context::ActorContext,
    message::{Handler, Message},
    system::ActorSystem,
    Actor, IntoActor,
};
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::{
    runtime::Runtime,
    sync::{OwnedSemaphorePermit, Semaphore},
};

struct UnboundedCounter {
    count: i64,
}

impl Actor for UnboundedCounter {}

struct Inc {
    amount: i64,
    #[allow(dead_code)]
    permit: Option<OwnedSemaphorePermit>,
}

impl Message for Inc {
    type Result = i64;
}

#[async_trait::async_trait]
impl Handler<Inc> for UnboundedCounter {
    async fn handle(&mut self, msg: Inc, _ctx: &mut ActorContext) -> i64 {
        self.count += msg.amount;
        self.count
    }
}

fn benchmark_tell_unbounded(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let num_actors = 100;

    c.bench_function("Coerce Tell Unbounded", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let sys = ActorSystem::new();
            let mut actor_refs = Vec::with_capacity(num_actors);
            for _ in 0..num_actors {
                let actor_ref = UnboundedCounter { count: 0 }
                    .into_actor::<&'static str>(None, &sys)
                    .await
                    .unwrap();
                actor_ref
                    .send(Inc {
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
                    .notify(Inc {
                        amount: 1,
                        permit: Some(permit),
                    })
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

    c.bench_function("Coerce Actor Creation", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let sys = ActorSystem::new();
            let start = Instant::now();

            for _ in 0..iters {
                let _ = UnboundedCounter { count: 0 }
                    .into_actor::<&'static str>(None, &sys)
                    .await
                    .unwrap();
            }

            start.elapsed()
        })
    });
}

criterion_group!(coerce, benchmark_tell_unbounded, benchmark_actor_creation,);
criterion_main!(coerce);
