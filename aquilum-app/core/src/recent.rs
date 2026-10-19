use std::borrow::Borrow;
use std::collections::VecDeque;

pub struct Recent<K, V> {
    items: VecDeque<(K, V, usize)>,
    count: usize,
    bytes: usize,
}

impl<K: PartialEq, V> Recent<K, V> {
    pub const fn new(count: usize, bytes: usize) -> Self {
        Recent { items: VecDeque::new(), count, bytes }
    }

    pub fn get<Q: PartialEq + ?Sized>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.find(|k| k.borrow() == key)
    }

    pub fn find(&mut self, matches: impl Fn(&K) -> bool) -> Option<&V> {
        let at = self.items.iter().position(|(k, _, _)| matches(k))?;
        let item = self.items.remove(at)?;
        self.items.push_back(item);
        self.items.back().map(|(_, value, _)| value)
    }

    pub fn put(&mut self, key: K, value: V, size: usize) {
        self.items.retain(|(k, _, _)| *k != key);
        self.items.push_back((key, value, size));
        let mut total: usize = self.items.iter().map(|(_, _, size)| size).sum();
        while self.items.len() > 1 && (self.items.len() > self.count || total > self.bytes) {
            if let Some((_, _, size)) = self.items.pop_front() {
                total -= size;
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.items.iter().map(|(key, value, _)| (key, value))
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&K, &V) -> bool) {
        self.items.retain(|(key, value, _)| keep(key, value));
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::Recent;

    #[test]
    fn the_oldest_entries_leave_first_and_one_always_stays() {
        let mut recent = Recent::new(2, 10);
        recent.put('a', 1, 4);
        recent.put('b', 2, 4);
        assert_eq!(recent.get(&'a'), Some(&1));
        recent.put('c', 3, 4);
        assert_eq!(recent.iter().map(|(k, _)| *k).collect::<String>(), "ac");
        recent.put('d', 4, 40);
        assert_eq!(recent.iter().map(|(k, _)| *k).collect::<String>(), "d");
    }
}
