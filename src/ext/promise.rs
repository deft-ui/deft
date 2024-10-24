use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Clone)]
struct PromiseInner<T, E> {
    value: Option<Result<T, E>>,
    waker: Option<Waker>,
}
#[derive(Clone)]
pub struct Promise<T, E> {
    inner: Arc<Mutex<PromiseInner<T, E>>>,
}

impl<T, E> Promise<T, E> {

    pub fn new() -> Self {
        let inner: PromiseInner<T, E> = PromiseInner {
            value: None,
            waker: None,
        };
        Promise {
            inner: Arc::new(Mutex::new(inner))
        }
    }

    pub fn new_resolved(value: T) -> Self {
        let p = Self::new();
        p.resolve(value);
        p
    }

    pub fn new_rejected(err: E) -> Self {
        let p = Self::new();
        p.reject(err);
        p
    }

    pub fn resolve(&self, value: T) {
        self.set_value(Ok(value));
    }

    pub fn reject(&self, err: E) {
        self.set_value(Err(err));
    }

    fn set_value(&self, result: Result<T, E>) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(v) = &inner.value {
            return;
        }
        inner.value = Some(result);
        if let Some(wk) = &inner.waker {
            wk.clone().wake();
        }
    }

}

impl<T, E> Future for Promise<T, E> where T: Clone, E: Clone {
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.inner.lock().unwrap();
        return if let Some(result) = &inner.value {
            match result {
                Ok(v) => { Poll::Ready(Ok(v.clone())) }
                Err(e) => { Poll::Ready(Err(e.clone())) }
            }
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}