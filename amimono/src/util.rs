use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, LazyLock, Mutex},
};

pub struct StaticHashMap<K, V: ?Sized> {
    inner: LazyLock<Mutex<HashMap<K, Arc<V>>>>,
}

impl<K, V: ?Sized> StaticHashMap<K, V> {
    pub const fn new() -> StaticHashMap<K, V> {
        StaticHashMap {
            inner: LazyLock::new(|| Mutex::new(HashMap::new())),
        }
    }
}

impl<K: Hash + Eq, V: ?Sized> StaticHashMap<K, V> {
    pub fn get<Q>(&self, q: &Q) -> Option<Arc<V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.lock().expect("lock poisoned").get(q).cloned()
    }

    pub fn insert(&self, k: K, v: Arc<V>) -> Option<Arc<V>> {
        self.inner.lock().expect("lock poisoned").insert(k, v)
    }
}

impl<K: Hash + Eq, V: Default> StaticHashMap<K, V> {
    pub fn get_or_insert(&self, k: K) -> Arc<V> {
        self.inner
            .lock()
            .expect("lock poisoned")
            .entry(k)
            .or_insert_with(|| Arc::new(Default::default()))
            .clone()
    }
}
