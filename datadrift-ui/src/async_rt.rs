//! Tiny current-thread Tokio runtime for async native drivers in the worker thread.

use std::future::Future;

pub fn block_on<F: Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(fut)
}
