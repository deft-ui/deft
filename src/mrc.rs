use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe};

struct MrcBox<T> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

pub struct Mrc<T> {
    ptr: *mut MrcBox<T>,
}

impl<T> PartialEq for Mrc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

//TODO fix typo
pub struct MrcWeak<T> {
    ptr: *mut MrcBox<T>,
}

//TODO should impl it?
impl<T> RefUnwindSafe for Mrc<T> {}

impl<T> Mrc<T> {
    pub fn new(value: T) -> Self {
        let ptr = Box::into_raw(Box::new(MrcBox {
            strong: Cell::new(1),
            weak: Cell::new(0),
            value,
        }));
        Mrc {
            ptr
        }
    }

    pub fn as_weak(&self) -> MrcWeak<T> {
        let weak = self.inner().weak.get();
        self.inner().weak.set(weak + 1);
        MrcWeak {
            ptr: self.ptr
        }
    }

    fn inner(&self) -> &mut MrcBox<T> {
        unsafe { &mut (*self.ptr) }
    }

}

impl<T> Deref for Mrc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &(*self.ptr).value
        }
    }
}

impl<T> DerefMut for Mrc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.ptr).value }
    }
}

impl<T> Clone for Mrc<T> {
    fn clone(&self) -> Self {
        unsafe {
            let strong = &(*self.ptr).strong;
            strong.set(strong.get() + 1);
            Self {
                ptr: self.ptr
            }
        }
    }
}

impl<T> MrcWeak<T> {

    pub fn upgrade(&self) -> Option<Mrc<T>> {
        let strong = self.inner().strong.get();
        if strong == 0 {
            return None
        }
        self.inner().strong.set(strong + 1);
        Some(Mrc {
            ptr: self.ptr,
        })
    }

    fn inner(&self) -> &mut MrcBox<T> {
        unsafe { &mut (*self.ptr) }
    }

}

impl<T> Clone for MrcWeak<T> {
    fn clone(&self) -> Self {
        let weak = self.inner().weak.get();
        self.inner().weak.set(weak + 1);
        Self {
            ptr: self.ptr,
        }
    }
}

impl<T> Drop for Mrc<T> {
    fn drop(&mut self) {
        let inner = self.inner();
        let strong = inner.strong.get() - 1;
        inner.strong.set(strong);
        if strong == 0 {
            unsafe {
                let _ = Box::from_raw(self.ptr);
            }
        }
    }
}