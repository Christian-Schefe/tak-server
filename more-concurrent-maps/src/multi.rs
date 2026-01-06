use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;

pub struct ConcurrentMultiMap<L, R> {
    inner: RwLock<MultiMap<L, R>>,
}

impl<L, R> ConcurrentMultiMap<L, R>
where
    L: std::hash::Hash + Eq + Clone,
    R: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(MultiMap::new()),
        }
    }

    pub fn get_by_left(&self, left: &L) -> Vec<R> {
        let map = self.inner.read();
        map.get_by_left(left).cloned().collect()
    }

    pub fn get_by_right(&self, right: &R) -> Vec<L> {
        let map = self.inner.read();
        map.get_by_right(right).cloned().collect()
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

    pub fn insert(&self, left: L, right: R) -> bool {
        let mut map = self.inner.write();
        map.insert(left, right)
    }

    pub fn remove(&self, left: &L, right: &R) -> bool {
        let mut map = self.inner.write();
        map.remove(left, right)
    }

    pub fn remove_by_left(&self, left: &L) -> Vec<R> {
        let mut map = self.inner.write();
        map.remove_by_left(left).collect()
    }

    pub fn remove_by_right(&self, right: &R) -> Vec<L> {
        let mut map = self.inner.write();
        map.remove_by_right(right).collect()
    }
}

pub struct MultiMap<L, R> {
    forward: HashMap<L, HashSet<R>>,
    backward: HashMap<R, HashSet<L>>,
    empty_forward: HashSet<R>,
    empty_backward: HashSet<L>,
}

#[allow(unused)]
impl<L, R> MultiMap<L, R>
where
    L: std::hash::Hash + Eq + Clone,
    R: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            backward: HashMap::new(),
            empty_forward: HashSet::new(),
            empty_backward: HashSet::new(),
        }
    }

    pub fn get_by_left(&self, left: &L) -> impl Iterator<Item = &R> {
        if let Some(set) = self.forward.get(left) {
            set.iter()
        } else {
            self.empty_forward.iter()
        }
    }

    pub fn get_by_right(&self, right: &R) -> impl Iterator<Item = &L> {
        if let Some(set) = self.backward.get(right) {
            set.iter()
        } else {
            self.empty_backward.iter()
        }
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

    pub fn insert(&mut self, left: L, right: R) -> bool {
        let forward_set = self.forward.entry(left.clone()).or_default();
        let backward_set = self.backward.entry(right.clone()).or_default();
        let inserted = forward_set.insert(right);
        backward_set.insert(left);
        inserted
    }

    pub fn remove(&mut self, left: &L, right: &R) -> bool {
        let mut removed = false;
        if let Some(forward_set) = self.forward.get_mut(left) {
            if forward_set.remove(right) {
                removed = true;
                if forward_set.is_empty() {
                    self.forward.remove(left);
                }
            }
        }
        if let Some(backward_set) = self.backward.get_mut(right) {
            backward_set.remove(left);
            if backward_set.is_empty() {
                self.backward.remove(right);
            }
        }
        removed
    }

    pub fn remove_by_left(&mut self, left: &L) -> impl Iterator<Item = R> {
        if let Some(forward_set) = self.forward.remove(left) {
            for right in &forward_set {
                if let Some(backward_set) = self.backward.get_mut(right) {
                    backward_set.remove(left);
                    if backward_set.is_empty() {
                        self.backward.remove(right);
                    }
                }
            }
            return forward_set.into_iter();
        }
        self.empty_forward.clone().into_iter()
    }

    pub fn remove_by_right(&mut self, right: &R) -> impl Iterator<Item = L> {
        if let Some(backward_set) = self.backward.remove(right) {
            for left in &backward_set {
                if let Some(forward_set) = self.forward.get_mut(left) {
                    forward_set.remove(right);
                    if forward_set.is_empty() {
                        self.forward.remove(left);
                    }
                }
            }
            return backward_set.into_iter();
        }
        self.empty_backward.clone().into_iter()
    }
}
