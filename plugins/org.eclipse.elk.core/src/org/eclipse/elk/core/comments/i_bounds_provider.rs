use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::cache_key::CacheKey;
use crate::org::eclipse::elk::core::math::ElkRectangle;

pub trait IBoundsProvider<C: 'static, T: 'static> {
    fn bounds_for_comment(&self, comment: &C) -> Option<ElkRectangle>;
    fn bounds_for_target(&self, target: &T) -> Option<ElkRectangle>;

    fn preprocess(
        &self,
        _data_provider: &dyn super::i_data_provider::IDataProvider<C, T>,
        _include_hierarchy: bool,
    ) {
    }

    fn cleanup(&self) {}

    fn cached(self: Rc<Self>) -> Rc<dyn IBoundsProvider<C, T>>
    where
        Self: Sized + 'static,
        C: CacheKey,
        T: CacheKey,
    {
        Rc::new(CachedBoundsProvider::new(self))
    }
}

struct CachedBoundsProvider<C: CacheKey, T: CacheKey> {
    inner: Rc<dyn IBoundsProvider<C, T>>,
    comment_bounds_cache: RefCell<HashMap<usize, Option<ElkRectangle>>>,
    target_bounds_cache: RefCell<HashMap<usize, Option<ElkRectangle>>>,
}

impl<C: CacheKey, T: CacheKey> CachedBoundsProvider<C, T> {
    fn new(inner: Rc<dyn IBoundsProvider<C, T>>) -> Self {
        CachedBoundsProvider {
            inner,
            comment_bounds_cache: RefCell::new(HashMap::new()),
            target_bounds_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl<C: CacheKey + 'static, T: CacheKey + 'static> IBoundsProvider<C, T>
    for CachedBoundsProvider<C, T>
{
    fn bounds_for_comment(&self, comment: &C) -> Option<ElkRectangle> {
        let key = comment.cache_key();
        let mut cache = self.comment_bounds_cache.borrow_mut();
        if let Some(bounds) = cache.get(&key) {
            return *bounds;
        }
        let bounds = self.inner.bounds_for_comment(comment);
        cache.insert(key, bounds);
        bounds
    }

    fn bounds_for_target(&self, target: &T) -> Option<ElkRectangle> {
        let key = target.cache_key();
        let mut cache = self.target_bounds_cache.borrow_mut();
        if let Some(bounds) = cache.get(&key) {
            return *bounds;
        }
        let bounds = self.inner.bounds_for_target(target);
        cache.insert(key, bounds);
        bounds
    }

    fn preprocess(
        &self,
        data_provider: &dyn super::i_data_provider::IDataProvider<C, T>,
        include_hierarchy: bool,
    ) {
        self.inner.preprocess(data_provider, include_hierarchy);
    }

    fn cleanup(&self) {
        self.comment_bounds_cache.borrow_mut().clear();
        self.target_bounds_cache.borrow_mut().clear();
        self.inner.cleanup();
    }
}
