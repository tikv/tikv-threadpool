// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

//! Metrics of the thread pool.

use lazy_static::lazy_static;
use prometheus::*;
use std::sync::Mutex;

lazy_static! {
    /// Elapsed time of each level in the multilevel task queue.
    pub static ref MULTILEVEL_LEVEL_ELAPSED: IntCounterVec = IntCounterVec::new(
        new_opts(
            "multilevel_level_elapsed",
            "elapsed time of each level in the multilevel task queue"
        ),
        &["name", "level"]
    )
    .unwrap();

    /// The chance that a level 0 task is scheduled to run.
    pub static ref MULTILEVEL_LEVEL0_CHANCE: GaugeVec = GaugeVec::new(
        new_opts(
            "multilevel_level0_chance",
            "the chance that a level 0 task is scheduled to run"
        ),
        &["name"]
    )
    .unwrap();

    /// The count of active workers.
    pub static ref ACTIVE_WORKERS_COUNT: HistogramVec = HistogramVec::new(
        histogram_opts!(
            "yatp_active_workers_count",
            "the count of backup workers",
            vec![1.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0]
        ),
        &["name"]
    )
    .unwrap();

    static ref NAMESPACE: Mutex<Option<String>> = Mutex::new(None);
}

/// Sets the namespace used in the metrics. This function should be called before
/// the metrics are used or any thread pool is created.
///
/// The namespace is missing by default.
pub fn set_namespace(s: Option<impl Into<String>>) {
    *NAMESPACE.lock().unwrap() = s.map(Into::into)
}

fn new_opts(name: &str, help: &str) -> Opts {
    let mut opts = Opts::new(name, help);
    if let Some(ref namespace) = *NAMESPACE.lock().unwrap() {
        opts = opts.namespace(namespace);
    }
    opts
}
