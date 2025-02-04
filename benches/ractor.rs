use std::{sync::Arc, time::Instant};

use criterion::{criterion_group, criterion_main, Criterion};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tokio::{
    runtime::Runtime,
    sync::{OwnedSemaphorePermit, Semaphore},
};

struct UnboundedCounter;

impl Actor for UnboundedCounter {
    type Msg = Inc;
    type State = i64;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(0)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        *state += message.amount;
        if let Some(tx) = message.reply {
            tx.send(*state)?;
        }
        Ok(())
    }
}

struct Inc {
    amount: i64,
    #[allow(dead_code)]
    permit: Option<OwnedSemaphorePermit>,
    reply: Option<RpcReplyPort<i64>>,
}

fn benchmark_tell_unbounded(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let num_actors = 100;

    c.bench_function("Ractor Tell Unbounded", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut actor_refs = Vec::with_capacity(num_actors);
            for _ in 0..num_actors {
                let (actor_ref, _) = Actor::spawn(None, UnboundedCounter, ()).await.unwrap();
                let _ = actor_ref
                    .call(
                        |tx| Inc {
                            amount: 0,
                            permit: None,
                            reply: Some(tx),
                        },
                        None,
                    )
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
                    .cast(Inc {
                        amount: 1,
                        permit: Some(permit),
                        reply: None,
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

    c.bench_function("Ractor Actor Creation", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let start = Instant::now();

            for _ in 0..iters {
                let _ = Actor::spawn(None, UnboundedCounter, ()).await.unwrap();
            }

            start.elapsed()
        })
    });
}

criterion_group!(ractor, benchmark_tell_unbounded, benchmark_actor_creation,);
criterion_main!(ractor);
