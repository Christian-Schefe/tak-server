use std::collections::HashMap;

use parking_lot::RwLock;

pub struct ConcurrentBiMap<L, R> {
    inner: RwLock<BiMap<L, R>>,
}

impl<L, R> ConcurrentBiMap<L, R>
where
    L: std::hash::Hash + Eq + Clone,
    R: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(BiMap::new()),
        }
    }

    pub fn get_by_left(&self, left: &L) -> Option<R> {
        let map = self.inner.read();
        map.get_by_left(left).cloned()
    }

    pub fn get_by_right(&self, right: &R) -> Option<L> {
        let map = self.inner.read();
        map.get_by_right(right).cloned()
    }

    pub fn get_left_keys(&self) -> Vec<L> {
        let map = self.inner.read();
        map.get_left_keys().cloned().collect()
    }

    pub fn get_right_keys(&self) -> Vec<R> {
        let map = self.inner.read();
        map.get_right_keys().cloned().collect()
    }

    pub fn contains_left(&self, left: &L) -> bool {
        let map = self.inner.read();
        map.contains_left(left)
    }

    pub fn contains_right(&self, right: &R) -> bool {
        let map = self.inner.read();
        map.contains_right(right)
    }

    pub fn try_insert(&self, left: L, right: R) -> bool {
        let mut map = self.inner.write();
        map.try_insert(left, right)
    }

    pub fn try_remove(&self, left: &L, right: &R) -> bool {
        let mut map = self.inner.write();
        map.try_remove(left, right)
    }

    pub fn remove_by_left(&self, left: &L) -> Option<R> {
        let mut map = self.inner.write();
        map.remove_by_left(left)
    }

    pub fn remove_by_right(&self, right: &R) -> Option<L> {
        let mut map = self.inner.write();
        map.remove_by_right(right)
    }
}

pub struct BiMap<L, R> {
    forward: HashMap<L, R>,
    backward: HashMap<R, L>,
}

#[allow(unused)]
impl<L, R> BiMap<L, R>
where
    L: std::hash::Hash + Eq + Clone,
    R: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            backward: HashMap::new(),
        }
    }

    pub fn get_by_left(&self, left: &L) -> Option<&R> {
        self.forward.get(left)
    }

    pub fn get_by_right(&self, right: &R) -> Option<&L> {
        self.backward.get(right)
    }

    pub fn get_left_keys(&self) -> impl Iterator<Item = &L> {
        self.forward.keys()
    }

    pub fn get_right_keys(&self) -> impl Iterator<Item = &R> {
        self.backward.keys()
    }

    pub fn contains_left(&self, left: &L) -> bool {
        self.forward.contains_key(left)
    }

    pub fn contains_right(&self, right: &R) -> bool {
        self.backward.contains_key(right)
    }

    pub fn try_insert(&mut self, left: L, right: R) -> bool {
        if self.forward.contains_key(&left) || self.backward.contains_key(&right) {
            return false;
        }
        self.forward.insert(left.clone(), right.clone());
        self.backward.insert(right, left);
        true
    }

    pub fn try_remove(&mut self, left: &L, right: &R) -> bool {
        let mut removed = false;
        if let Some(v) = self.forward.get(left) {
            if v == right {
                self.forward.remove(left);
                self.backward.remove(right);
                removed = true;
            }
        }
        removed
    }

    pub fn remove_by_left(&mut self, left: &L) -> Option<R> {
        if let Some(right) = self.forward.remove(left) {
            self.backward.remove(&right);
            return Some(right);
        }
        None
    }

    pub fn remove_by_right(&mut self, right: &R) -> Option<L> {
        if let Some(left) = self.backward.remove(right) {
            self.forward.remove(&left);
            return Some(left);
        }
        None
    }
}
