use std::time::Instant;

use actix::{Actor, Context, Handler, Message, System, SystemRunner};
use criterion::{async_executor::AsyncExecutor, criterion_group, criterion_main, Criterion};

struct ActixRuntime(SystemRunner);

impl AsyncExecutor for &ActixRuntime {
    fn block_on<T>(&self, future: impl std::future::Future<Output = T>) -> T {
        self.0.block_on(future)
    }
}

struct BoundedCounter {
    count: i64,
}

impl Actor for BoundedCounter {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(1_000_000);
    }
}

#[derive(Message)]
#[rtype(result = "i64")]
struct Inc {
    amount: i64,
}

impl Handler<Inc> for BoundedCounter {
    type Result = i64;

    fn handle(&mut self, msg: Inc, _ctx: &mut Context<Self>) -> Self::Result {
        self.count += msg.amount;
        self.count
    }
}

fn benchmark_tell_bounded(c: &mut Criterion) {
    let rt = ActixRuntime(System::new());
    let num_actors = 100;

    c.bench_function("Actix Tell Bounded", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let mut actor_refs = Vec::with_capacity(num_actors);
            for _ in 0..num_actors {
                let actor_ref = BoundedCounter { count: 0 }.start();
                actor_ref.send(Inc { amount: 0 }).await.unwrap();
                actor_refs.push(actor_ref);
            }

            let start = Instant::now();

            for i in 0..iters {
                let actor_ref = &actor_refs[i as usize % num_actors];
                actor_ref.try_send(Inc { amount: 1 }).unwrap();
            }

            start.elapsed()
        })
    });
}

fn benchmark_actor_creation(c: &mut Criterion) {
    let rt = ActixRuntime(System::new());

    c.bench_function("Actix Actor Creation", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let start = Instant::now();

            for _ in 0..iters {
                let _ = BoundedCounter { count: 0 }.start();
            }

            start.elapsed()
        })
    });
}

criterion_group!(actix, benchmark_tell_bounded, benchmark_actor_creation);
criterion_main!(actix);
