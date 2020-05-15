// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use criterion::*;

mod yatp_callback {
    use criterion::*;
    use std::sync::atomic::*;
    use std::sync::*;
    use yatp::task::callback::Handle;

    pub fn ping_pong(b: &mut Bencher<'_>, ping_count: usize) {
        let pool = yatp::Builder::new("ping_pong").build_callback_pool();

        let (done_tx, done_rx) = mpsc::sync_channel(1000);
        let rem = Arc::new(AtomicUsize::new(0));

        b.iter(|| {
            let done_tx = done_tx.clone();
            let rem = rem.clone();
            rem.store(ping_count, Ordering::Relaxed);

            pool.spawn(move |h: &mut Handle<'_>| {
                for _ in 0..ping_count {
                    let rem = rem.clone();
                    let done_tx = done_tx.clone();

                    h.spawn(move |h: &mut Handle<'_>| {
                        let (tx1, rx1) = mpsc::channel();
                        let (tx2, rx2) = mpsc::channel();

                        tx1.send(()).unwrap();
                        h.spawn(move |h: &mut Handle<'_>| {
                            rx1.recv().unwrap();
                            tx2.send(()).unwrap();
                            h.spawn(move |_: &mut Handle<'_>| {
                                rx2.recv().unwrap();
                                if 1 == rem.fetch_sub(1, Ordering::Relaxed) {
                                    done_tx.send(()).unwrap();
                                }
                            })
                        })
                    });
                }
            });

            done_rx.recv().unwrap();
        });
    }
}

mod yatp_future {
    use criterion::*;
    use std::sync::atomic::*;
    use std::sync::*;
    use tokio::sync::oneshot;
    use yatp::task::future::TaskCell;

    fn ping_pong(b: &mut Bencher<'_>, pool: yatp::ThreadPool<TaskCell>, ping_count: usize) {
        let (done_tx, done_rx) = mpsc::sync_channel(1000);
        let rem = Arc::new(AtomicUsize::new(0));

        b.iter(|| {
            let done_tx = done_tx.clone();
            let rem = rem.clone();
            rem.store(ping_count, Ordering::Relaxed);

            let remote = pool.remote().clone();

            pool.spawn(async move {
                for _ in 0..ping_count {
                    let rem = rem.clone();
                    let done_tx = done_tx.clone();

                    let remote2 = remote.clone();

                    remote.spawn(async move {
                        let (tx1, rx1) = oneshot::channel();
                        let (tx2, rx2) = oneshot::channel();

                        remote2.spawn(async move {
                            rx1.await.unwrap();
                            tx2.send(()).unwrap();
                        });

                        tx1.send(()).unwrap();
                        rx2.await.unwrap();

                        if 1 == rem.fetch_sub(1, Ordering::Relaxed) {
                            done_tx.send(()).unwrap();
                        }
                    });
                }
            });

            done_rx.recv().unwrap();
        });
    }

    pub fn ping_pong_single_level(b: &mut Bencher<'_>, ping_count: usize) {
        let pool = yatp::Builder::new("ping_pong").build_future_pool();
        ping_pong(b, pool, ping_count)
    }

    pub fn ping_pong_multilevel(b: &mut Bencher<'_>, ping_count: usize) {
        let pool = yatp::Builder::new("ping_pong").build_multilevel_future_pool();
        ping_pong(b, pool, ping_count)
    }
}

mod tokio {
    use criterion::*;
    use std::sync::atomic::*;
    use std::sync::*;
    use tokio::runtime::Builder;
    use tokio::sync::oneshot;

    pub fn ping_pong(b: &mut Bencher<'_>, ping_count: usize) {
        let pool = Builder::new()
            .threaded_scheduler()
            .core_threads(num_cpus::get())
            .build()
            .unwrap();

        let (done_tx, done_rx) = mpsc::sync_channel(1000);
        let rem = Arc::new(AtomicUsize::new(0));

        b.iter(|| {
            let done_tx = done_tx.clone();
            let rem = rem.clone();
            rem.store(ping_count, Ordering::Relaxed);

            let handle = pool.handle().clone();

            pool.spawn(async move {
                for _ in 0..ping_count {
                    let rem = rem.clone();
                    let done_tx = done_tx.clone();

                    let handle2 = handle.clone();

                    handle.spawn(async move {
                        let (tx1, rx1) = oneshot::channel();
                        let (tx2, rx2) = oneshot::channel();

                        handle2.spawn(async move {
                            rx1.await.unwrap();
                            tx2.send(()).unwrap();
                        });

                        tx1.send(()).unwrap();
                        rx2.await.unwrap();

                        if 1 == rem.fetch_sub(1, Ordering::Relaxed) {
                            done_tx.send(()).unwrap();
                        }
                    });
                }
            });

            done_rx.recv().unwrap();
        });
    }
}

mod async_std {
    use criterion::*;
    use std::sync::atomic::*;
    use std::sync::*;
    use tokio::sync::oneshot;

    pub fn ping_pong(b: &mut Bencher<'_>, ping_count: usize) {
        let (done_tx, done_rx) = mpsc::sync_channel(1000);
        let rem = Arc::new(AtomicUsize::new(0));

        b.iter(|| {
            let done_tx = done_tx.clone();
            let rem = rem.clone();
            rem.store(ping_count, Ordering::Relaxed);

            async_std::task::spawn(async move {
                for _ in 0..ping_count {
                    let rem = rem.clone();
                    let done_tx = done_tx.clone();

                    async_std::task::spawn(async move {
                        let (tx1, rx1) = oneshot::channel();
                        let (tx2, rx2) = oneshot::channel();

                        async_std::task::spawn(async move {
                            rx1.await.unwrap();
                            tx2.send(()).unwrap();
                        });

                        tx1.send(()).unwrap();
                        rx2.await.unwrap();

                        if 1 == rem.fetch_sub(1, Ordering::Relaxed) {
                            done_tx.send(()).unwrap();
                        }
                    });
                }
            });

            done_rx.recv().unwrap();
        });
    }
}

pub fn ping_pong(b: &mut Criterion) {
    let mut group = b.benchmark_group("ping_pong");
    for i in &[256, 512, 1024] {
        group.bench_with_input(BenchmarkId::new("yatp::future", i), i, |b, i| {
            yatp_future::ping_pong_single_level(b, *i)
        });
        group.bench_with_input(BenchmarkId::new("yatp::callback", i), i, |b, i| {
            yatp_callback::ping_pong(b, *i)
        });
        group.bench_with_input(
            BenchmarkId::new("yatp::future::multilevel", i),
            i,
            |b, i| yatp_future::ping_pong_multilevel(b, *i),
        );
        group.bench_with_input(BenchmarkId::new("tokio", i), i, |b, i| {
            tokio::ping_pong(b, *i)
        });
        group.bench_with_input(BenchmarkId::new("async-std", i), i, |b, i| {
            async_std::ping_pong(b, *i)
        });
    }
    group.finish();
}

criterion_group!(ping_pong_group, ping_pong);

criterion_main!(ping_pong_group);
