use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub trait Cache<K, V> {
    fn new(capacity: usize) -> Self;
    fn put(&mut self, key: K, value: V);
    fn get(&mut self, key: &K) -> Option<&V>;
    fn len(&self) -> usize;
}

#[derive(Debug)]
pub(crate) struct Node<K, V> {
    pub(crate) key: K,
    pub(crate) value: V,
    pub(crate) prev: Option<usize>,
    pub(crate) next: Option<usize>,
}

pub struct LruCache<K, V> {
    capacity: usize,
    pub(crate) map: HashMap<K, usize>,
    pub(crate) arena: Vec<Node<K, V>>,
    pub head: Option<usize>,
    pub(crate) tail: Option<usize>,
}

impl<K, V> Cache<K, V> for LruCache<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Debug,
{
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "La capacité doit être > 0");
        LruCache {
            capacity,
            map: HashMap::with_capacity(capacity),
            arena: Vec::with_capacity(capacity),
            head: None,
            tail: None,
        }
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(&index) = self.map.get(key) {
            self.move_to_head(index);
            return Some(&self.arena[index].value);
        }
        None
    }

    fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            let index = self.map[&key];
            self.arena[index].value = value;
            self.move_to_head(index);
        } else {
            if self.arena.len() >= self.capacity {
                self.remove_lru();
            }

            let index = self.arena.len();
            let node = Node {
                key: key.clone(),
                value,
                prev: None,
                next: self.head,
            };

            self.arena.push(node);
            self.map.insert(key, index);

            if let Some(old_head_idx) = self.head {
                self.arena[old_head_idx].prev = Some(index);
            }

            self.head = Some(index);

            if self.tail.is_none() {
                self.tail = Some(index);
            }
        }
    }

    fn len(&self) -> usize {
        self.arena.len()
    }
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone + Debug,
{
    fn move_to_head(&mut self, index: usize) {
        if Some(index) == self.head {
            return;
        }
        let prev_idx = self.arena[index].prev;
        let next_idx = self.arena[index].next;
        if let Some(prev) = prev_idx {
            self.arena[prev].next = next_idx;
        }
        if let Some(next) = next_idx {
            self.arena[next].prev = prev_idx;
        }
        if Some(index) == self.tail {
            self.tail = prev_idx;
        }
        if let Some(old_head) = self.head {
            self.arena[old_head].prev = Some(index);
        }
        self.arena[index].next = self.head;
        self.arena[index].prev = None;
        self.head = Some(index);
    }

    fn remove_lru(&mut self) {
        if let Some(tail_idx) = self.tail {
            let key_to_remove = self.arena[tail_idx].key.clone();
            self.map.remove(&key_to_remove);

            self.tail = self.arena[tail_idx].prev;
            if let Some(new_tail) = self.tail {
                self.arena[new_tail].next = None;
            } else {
                self.head = None;
            }

            if tail_idx < self.arena.len() {
                let moved_key = self.arena[tail_idx].key.clone();
                self.map.insert(moved_key, tail_idx);
                let prev = self.arena[tail_idx].prev;
                let next = self.arena[tail_idx].next;

                if let Some(p) = prev {
                    self.arena[p].next = Some(tail_idx);
                }
                if let Some(n) = next {
                    self.arena[n].prev = Some(tail_idx);
                }

                if self.head == Some(self.arena.len()) {
                    self.head = Some(tail_idx);
                }
                if self.tail == Some(self.arena.len()) {
                    self.tail = Some(tail_idx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_put_get() {
        let mut cache = LruCache::new(2);
        cache.put("A", 1);
        cache.put("B", 2);
        assert_eq!(cache.get(&"A"), Some(&1));
        assert_eq!(cache.get(&"B"), Some(&2));
    }

    #[test]
    fn test_eviction_lru() {
        let mut cache = LruCache::new(2);
        cache.put("A", 1);
        cache.put("B", 2);
        cache.get(&"A");
        cache.put("C", 3);
        assert_eq!(cache.get(&"B"), None);
        assert_eq!(cache.get(&"A"), Some(&1));
        assert_eq!(cache.get(&"C"), Some(&3));
    }
}
