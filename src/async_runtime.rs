use std::future::Future;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;
thread_local! {
    pub static ASYNC_RUNTIME: Runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    ASYNC_RUNTIME.with(|e| {
        e.spawn(future)
    })
}