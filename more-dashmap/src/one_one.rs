use std::sync::Mutex;

use dashmap::DashMap;

pub struct OneOneDashMap<K, V> {
    forward: DashMap<K, V>,
    backward: DashMap<V, K>,
    lock: Mutex<()>,
}

#[allow(unused)]
impl<K, V> OneOneDashMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            forward: DashMap::new(),
            backward: DashMap::new(),
            lock: Mutex::new(()),
        }
    }

    pub fn get_by_key(&self, key: &K) -> Option<V> {
        self.forward.get(key).map(|v| v.clone())
    }

    pub fn get_by_value(&self, value: &V) -> Option<K> {
        self.backward.get(value).map(|k| k.clone())
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.forward.contains_key(key)
    }

    pub fn contains_value(&self, value: &V) -> bool {
        self.backward.contains_key(value)
    }

    pub fn try_insert(&self, key: K, value: V) -> bool {
        let _guard = self.lock.lock().unwrap();
        if self.forward.contains_key(&key) || self.backward.contains_key(&value) {
            return false;
        }
        self.forward.insert(key.clone(), value.clone());
        self.backward.insert(value, key);
        drop(_guard);
        true
    }

    pub fn try_remove(&self, key: &K, value: &V) -> bool {
        let _guard = self.lock.lock().unwrap();
        let mut removed = false;
        if let Some(v) = self.forward.get(key) {
            if &*v == value {
                drop(v);
                self.forward.remove(key);
                self.backward.remove(value);
                removed = true;
            }
        }
        drop(_guard);
        removed
    }

    pub fn remove_by_key(&self, key: &K) -> Option<V> {
        let _guard = self.lock.lock().unwrap();
        if let Some((_, value)) = self.forward.remove(key) {
            self.backward.remove(&value);
            return Some(value);
        }
        drop(_guard);
        None
    }

    pub fn remove_by_value(&self, value: &V) -> Option<K> {
        let _guard = self.lock.lock().unwrap();
        if let Some((_, key)) = self.backward.remove(value) {
            self.forward.remove(&key);
            return Some(key);
        }
        drop(_guard);
        None
    }

    pub fn get_keys(&self) -> Vec<K> {
        self.forward
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn get_values(&self) -> Vec<V> {
        self.backward
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}
