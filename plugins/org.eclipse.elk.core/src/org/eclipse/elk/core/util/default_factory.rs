use std::marker::PhantomData;

use crate::org::eclipse::elk::core::util::IFactory;

pub struct DefaultFactory<T> {
    _marker: PhantomData<T>,
}

impl<T> DefaultFactory<T> {
    pub fn new() -> Self {
        DefaultFactory {
            _marker: PhantomData,
        }
    }
}

impl<T> Default for DefaultFactory<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IFactory<T> for DefaultFactory<T>
where
    T: Default + Send + Sync,
{
    fn create(&self) -> T {
        T::default()
    }

    fn destroy(&self, _obj: T) {
        // no-op by default
    }
}
