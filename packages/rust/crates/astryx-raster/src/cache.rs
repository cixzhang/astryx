//! A bounded cache that keeps what is in use.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{BuildHasher, Hash, RandomState};

/// Holds about `limit` entries in two generations. New entries go into the
/// hot generation; when it is full it becomes the cold one and the old cold
/// one is dropped. A hit in the cold generation moves the entry back, so
/// entries in use stay.
///
/// The limit doubles (up to `max`) instead of dropping entries when the
/// entries in use do not fit:
/// - a second rotation within one frame (see [`Generational::next_frame`])
///   would drop entries made in that frame, which layout asks for again in
///   its later passes;
/// - an entry the cache dropped earlier is asked for again (the cache
///   remembers the hashes of dropped entries).
///
/// Entries that are never asked for again still go, so a stream of new
/// keys over many frames (a clock, a log) keeps the cache at about `limit`.
pub(crate) struct Generational<K, V> {
    hot: HashMap<K, V>,
    cold: HashMap<K, V>,
    /// Hashes of dropped entries, oldest first, at most `max`.
    dropped: VecDeque<u64>,
    dropped_set: HashSet<u64>,
    /// A dropped entry was asked for again since the last check.
    returned: bool,
    /// The current frame, and the frame the cold generation was made in.
    frame: u64,
    cold_frame: Option<u64>,
    limit: usize,
    max: usize,
    hasher: RandomState,
}

impl<K: Hash + Eq, V: Clone> Generational<K, V> {
    pub fn new(limit: usize, max: usize) -> Self {
        Generational {
            hot: HashMap::new(),
            cold: HashMap::new(),
            dropped: VecDeque::new(),
            dropped_set: HashSet::new(),
            returned: false,
            frame: 0,
            cold_frame: None,
            limit: limit.max(1),
            max: max.max(limit.max(1)),
            hasher: RandomState::new(),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(value) = self.hot.get(key) {
            return Some(value.clone());
        }
        let (key, value) = self.cold.remove_entry(key)?;
        self.put(key, value.clone());
        Some(value)
    }

    pub fn insert(&mut self, key: K, value: V) {
        if !self.dropped_set.is_empty() && self.dropped_set.contains(&self.hasher.hash_one(&key)) {
            self.returned = true;
        }
        self.put(key, value);
    }

    /// Starts a new frame: entries made from now on may push out those of
    /// earlier frames, but not each other.
    pub fn next_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    pub fn clear(&mut self) {
        self.hot.clear();
        self.cold.clear();
        self.dropped.clear();
        self.dropped_set.clear();
        self.returned = false;
    }

    /// Entries held in both generations.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.hot.len() + self.cold.len()
    }

    fn put(&mut self, key: K, value: V) {
        if self.hot.len() >= self.limit {
            let crowded = std::mem::take(&mut self.returned) || self.cold_frame == Some(self.frame);
            if crowded && self.limit < self.max {
                self.limit = (self.limit * 2).min(self.max);
            } else {
                self.rotate();
            }
        }
        self.hot.insert(key, value);
    }

    fn rotate(&mut self) {
        self.cold_frame = Some(self.frame);
        let gone = std::mem::replace(&mut self.cold, std::mem::take(&mut self.hot));
        for key in gone.keys() {
            let hash = self.hasher.hash_one(key);
            if self.dropped_set.insert(hash) {
                self.dropped.push_back(hash);
            }
        }
        while self.dropped.len() > self.max {
            if let Some(old) = self.dropped.pop_front() {
                self.dropped_set.remove(&old);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Generational;

    #[test]
    fn entries_in_use_stay_when_the_cache_fills() {
        let mut c = Generational::new(4, 64);
        for k in 0..4 {
            c.insert(k, k);
        }
        // 0 and 1 are used again; 4..8 are new, one frame each.
        for k in 4..8 {
            c.next_frame();
            assert_eq!(c.get(&0), Some(0));
            assert_eq!(c.get(&1), Some(1));
            c.insert(k, k);
        }
        assert_eq!(c.get(&0), Some(0));
        assert_eq!(c.get(&1), Some(1));
    }

    #[test]
    fn the_limit_grows_when_dropped_entries_come_back_and_not_otherwise() {
        // A cycle over 10 keys in a cache of 4, one frame per key: after
        // the first round the cache grows to hold them all.
        let mut c = Generational::new(4, 64);
        let mut misses = 0;
        for round in 0..4 {
            for k in 0..10 {
                c.next_frame();
                if c.get(&k).is_none() {
                    misses += 1;
                    c.insert(k, k);
                }
            }
            if round == 1 {
                misses = 0;
            }
        }
        assert_eq!(misses, 0, "rounds 3 and 4 hit every key");
        // Keys never asked for again, one frame each: bounded at two
        // generations.
        let mut s = Generational::new(4, 64);
        for k in 0..1000 {
            s.next_frame();
            s.insert(k, k);
        }
        assert!(s.len() <= 8, "{}", s.len());
    }

    #[test]
    fn nothing_made_in_a_frame_is_dropped_in_that_frame() {
        let mut c = Generational::new(4, 64);
        for k in 0..30 {
            c.insert(k, k);
        }
        for k in 0..30 {
            assert_eq!(c.get(&k), Some(k), "key {k}");
        }
    }

    #[test]
    fn the_limit_stops_at_its_maximum() {
        let mut c = Generational::new(2, 8);
        for _ in 0..5 {
            for k in 0..100 {
                if c.get(&k).is_none() {
                    c.insert(k, k);
                }
            }
        }
        assert!(c.len() <= 16, "{}", c.len());
    }
}
