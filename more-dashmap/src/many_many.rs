use std::collections::HashSet;

use dashmap::DashMap;

pub struct ManyManyDashMap<K, V> {
    forward: DashMap<K, HashSet<V>>,
    backward: DashMap<V, HashSet<K>>,
}

impl<K, V> ManyManyDashMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            forward: DashMap::new(),
            backward: DashMap::new(),
        }
    }

    pub fn insert(&self, key: K, value: V) {
        self.forward
            .entry(key.clone())
            .or_default()
            .insert(value.clone());
        self.backward.entry(value).or_default().insert(key);
    }

    pub fn remove(&self, key: &K, value: &V) -> bool {
        let mut removed = false;
        if let Some(mut values) = self.forward.get_mut(key) {
            removed = values.remove(value);
            if values.is_empty() {
                drop(values);
                self.forward.remove(key);
            }
        }
        if let Some(mut keys) = self.backward.get_mut(value) {
            keys.remove(key);
            if keys.is_empty() {
                drop(keys);
                self.backward.remove(value);
            }
        }
        removed
    }

    pub fn get_by_key(&self, key: &K) -> Vec<V> {
        if let Some(values) = self.forward.get(key) {
            return values.iter().cloned().collect();
        }
        Vec::new()
    }

    pub fn get_by_value(&self, value: &V) -> Vec<K> {
        if let Some(keys) = self.backward.get(value) {
            return keys.iter().cloned().collect();
        }
        Vec::new()
    }

    pub fn remove_key(&self, key: &K) -> Vec<V> {
        if let Some((_, values)) = self.forward.remove(key) {
            for value in &values {
                if let Some(mut keys) = self.backward.get_mut(value) {
                    keys.remove(key);
                    if keys.is_empty() {
                        drop(keys);
                        self.backward.remove(value);
                    }
                }
            }
            return values.into_iter().collect();
        }
        Vec::new()
    }

    pub fn remove_value(&self, value: &V) -> Vec<K> {
        if let Some((_, keys)) = self.backward.remove(value) {
            for key in &keys {
                if let Some(mut values) = self.forward.get_mut(key) {
                    values.remove(value);
                    if values.is_empty() {
                        drop(values);
                        self.forward.remove(key);
                    }
                }
            }
            return keys.into_iter().collect();
        }
        Vec::new()
    }
}
