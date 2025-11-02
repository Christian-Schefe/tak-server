use std::collections::HashSet;

use dashmap::DashMap;
use validator::Validate;

use crate::{ServiceError, ServiceResult};

#[derive(Validate)]
struct EmailValidator {
    #[validate(email)]
    email: String,
}

pub fn validate_email(email: &str) -> ServiceResult<String> {
    let validator = EmailValidator {
        email: email.trim().to_string(),
    };
    if let Err(e) = validator.validate() {
        return ServiceError::bad_request(format!("Invalid email: {}", e));
    }
    Ok(validator.email)
}

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

pub struct OneOneDashMap<K, V> {
    forward: DashMap<K, V>,
    backward: DashMap<V, K>,
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
        if self.forward.contains_key(&key) || self.backward.contains_key(&value) {
            return false;
        }
        self.forward.insert(key.clone(), value.clone());
        self.backward.insert(value, key);
        true
    }

    pub fn remove(&self, key: &K, value: &V) -> bool {
        let mut removed = false;
        if let Some(v) = self.forward.get(key) {
            if &*v == value {
                drop(v);
                self.forward.remove(key);
                self.backward.remove(value);
                removed = true;
            }
        }

        removed
    }

    pub fn remove_by_key(&self, key: &K) -> Option<V> {
        if let Some((_, value)) = self.forward.remove(key) {
            self.backward.remove(&value);
            return Some(value);
        }
        None
    }

    pub fn remove_by_value(&self, value: &V) -> Option<K> {
        if let Some((_, key)) = self.backward.remove(value) {
            self.forward.remove(&key);
            return Some(key);
        }
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
