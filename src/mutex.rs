use std::{
    fmt::Display,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Debug)]
pub struct Mut<T>(RwLock<T>);

impl<T> Mut<T> {
    pub const fn new(obj: T) -> Mut<T> {
        Mut(RwLock::new(obj))
    }

    pub fn lock_ro(&self) -> RwLockReadGuard<T> {
        self.0.read().unwrap()
    }

    pub fn lock(&self) -> RwLockWriteGuard<T> {
        self.0.write().unwrap()
    }
}

impl<T> Display for Mut<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.read().unwrap().fmt(f)
    }
}

impl<T> Clone for Mut<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(RwLock::new(self.0.read().unwrap().clone()))
    }
}

impl<T> PartialEq for Mut<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.read().unwrap().eq(&other.0.read().unwrap())
    }
}

impl<T> Eq for Mut<T> where T: Eq {}

impl<T> PartialOrd for Mut<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.read().unwrap().partial_cmp(&other.0.read().unwrap())
    }
}
