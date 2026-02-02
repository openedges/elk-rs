use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::org::eclipse::elk::core::comments::cache_key::CacheKey;

pub trait IDataProvider<C: Clone + 'static, T: Clone + 'static> {
    fn provide_comments(&self) -> Vec<C>;
    fn provide_targets(&self) -> Vec<T>;

    fn provide_targets_for(&self, _comment: &C) -> Vec<T> {
        self.provide_targets()
    }

    fn provide_sub_hierarchies(&self) -> Vec<ProviderRef<C, T>>;

    fn attach(&self, comment: &C, target: &T);

    fn cached(self: Rc<Self>) -> ProviderRef<C, T>
    where
        Self: Sized + 'static,
        C: CacheKey,
    {
        Rc::new(CachedDataProvider::new(self))
    }
}

type ProviderRef<C, T> = Rc<dyn IDataProvider<C, T>>;

struct CachedDataProvider<C: Clone + CacheKey, T: Clone> {
    inner: ProviderRef<C, T>,
    comments_cache: RefCell<Option<Vec<C>>>,
    targets_cache: RefCell<Option<Vec<T>>>,
    comment_targets_cache: RefCell<HashMap<usize, Vec<T>>>,
    sub_provider_cache: RefCell<Option<Vec<ProviderRef<C, T>>>>,
}

impl<C: Clone + CacheKey, T: Clone> CachedDataProvider<C, T> {
    fn new(inner: ProviderRef<C, T>) -> Self {
        CachedDataProvider {
            inner,
            comments_cache: RefCell::new(None),
            targets_cache: RefCell::new(None),
            comment_targets_cache: RefCell::new(HashMap::new()),
            sub_provider_cache: RefCell::new(None),
        }
    }
}

impl<C: Clone + CacheKey + 'static, T: Clone + 'static> IDataProvider<C, T>
    for CachedDataProvider<C, T>
{
    fn provide_comments(&self) -> Vec<C> {
        let mut cache = self.comments_cache.borrow_mut();
        if cache.is_none() {
            *cache = Some(self.inner.provide_comments());
        }
        cache.clone().unwrap_or_default()
    }

    fn provide_targets(&self) -> Vec<T> {
        let mut cache = self.targets_cache.borrow_mut();
        if cache.is_none() {
            *cache = Some(self.inner.provide_targets());
        }
        cache.clone().unwrap_or_default()
    }

    fn provide_targets_for(&self, comment: &C) -> Vec<T> {
        let key = comment.cache_key();
        let mut cache = self.comment_targets_cache.borrow_mut();
        if let Some(targets) = cache.get(&key) {
            return targets.clone();
        }
        let targets = self.inner.provide_targets_for(comment);
        cache.insert(key, targets.clone());
        targets
    }

    fn provide_sub_hierarchies(&self) -> Vec<ProviderRef<C, T>> {
        let mut cache = self.sub_provider_cache.borrow_mut();
        if cache.is_none() {
            let providers = self
                .inner
                .provide_sub_hierarchies()
                .into_iter()
                .map(|provider| Rc::new(CachedDataProvider::new(provider)) as ProviderRef<C, T>)
                .collect::<Vec<_>>();
            *cache = Some(providers);
        }
        cache.clone().unwrap_or_default()
    }

    fn attach(&self, comment: &C, target: &T) {
        self.inner.attach(comment, target);
    }
}
