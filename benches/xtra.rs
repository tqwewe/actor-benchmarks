use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use xtra::{Actor, Context, Handler, Mailbox};

#[derive(Actor)]
struct Counter {
    count: i64,
}

struct Inc {
    amount: i64,
}

impl Handler<Inc> for Counter {
    type Return = i64;

    async fn handle(&mut self, msg: Inc, _ctx: &mut Context<Self>) -> Self::Return {
        self.count += msg.amount;
        self.count
    }
}

fn benchmark_tell_unbounded(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let num_actors = 100;

    c.bench_function("Xtra Tell Unbounded", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut actor_refs = Vec::with_capacity(num_actors);
            for _ in 0..num_actors {
                let actor_ref = xtra::spawn_tokio(Counter { count: 0 }, Mailbox::unbounded());
                actor_ref.send(Inc { amount: 0 }).await.unwrap();
                actor_refs.push(actor_ref);
            }

            let start = Instant::now();

            for i in 0..iters {
                let actor_ref = &actor_refs[i as usize % num_actors];
                let _ = actor_ref.send(Inc { amount: 1 }).detach().await.unwrap();
            }

            start.elapsed()
        })
    });
}

fn benchmark_tell_bounded(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let num_actors = 100;

    c.bench_function("Xtra Tell Bounded", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut actor_refs = Vec::with_capacity(num_actors);
            for _ in 0..num_actors {
                let actor_ref =
                    xtra::spawn_tokio(Counter { count: 0 }, Mailbox::bounded(1_000_000));
                actor_ref.send(Inc { amount: 0 }).await.unwrap();
                actor_refs.push(actor_ref);
            }

            let start = Instant::now();

            for i in 0..iters {
                let actor_ref = &actor_refs[i as usize % num_actors];
                let _ = actor_ref.send(Inc { amount: 1 }).detach().await.unwrap();
            }

            start.elapsed()
        })
    });
}

fn benchmark_actor_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("Xtra Actor Creation", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let start = Instant::now();

            for _ in 0..iters {
                let _ = xtra::spawn_tokio(Counter { count: 0 }, Mailbox::unbounded());
            }

            start.elapsed()
        })
    });
}

criterion_group!(
    xtra,
    benchmark_tell_unbounded,
    benchmark_tell_bounded,
    benchmark_actor_creation,
);
criterion_main!(xtra);
