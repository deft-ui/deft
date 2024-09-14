use std::cell::RefCell;

pub struct CacheValue<P, R> {
    inner: RefCell<CacheValueInner<P, R>>,
}

struct CacheValueInner<P, R> {
    last_params: Option<P>,
    last_value: Option<R>,
    compute: Box<dyn Fn(&P) -> R>,
}

impl<P: PartialEq, R: Clone> CacheValue<P, R> {

    pub fn new<C: (Fn(&P) -> R) + 'static>(compute: C) -> Self {
        Self {
            inner: RefCell::new(CacheValueInner {
                last_value: None,
                last_params: None,
                compute: Box::new(compute),
            })
        }
    }

    pub fn get(&self, params: P) -> R {
        let mut inner = self.inner.borrow_mut();
        if inner.last_value.is_none() || Some(&params) != inner.last_params.as_ref() {
            inner.last_value = Some((inner.compute)(&params));
            inner.last_params = Some(params);
        }
        inner.last_value.as_ref().unwrap().clone()
    }
}