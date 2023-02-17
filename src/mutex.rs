use std::sync::{Mutex, MutexGuard};

pub struct Mut<T>(Mutex<T>);

impl<T> Mut<T> {
    pub fn new(obj: T) -> Mut<T> {
        Mut(Mutex::new(obj))
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.0.lock().unwrap()
    }
}

impl<T> Clone for Mut<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().unwrap().clone()))
    }
}
