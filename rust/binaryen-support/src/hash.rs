use ahash::AHashMap;

pub type FastHashMap<K, V> = AHashMap<K, V>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::StringInterner;

    #[test]
    fn test_fast_hashmap_basic() {
        let mut map: FastHashMap<String, usize> = FastHashMap::default();
        map.insert("one".to_string(), 1);
        map.insert("two".to_string(), 2);
        assert_eq!(map.get("one"), Some(&1));
        assert_eq!(map.get("none"), None);
    }
}
