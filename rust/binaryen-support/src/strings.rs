use std::collections::HashMap;
use std::sync::RwLock;

/// A simple string interner for efficient canonicalizing of string values.
/// This leaks memory intentionally for now to return &'static str references.
pub struct StringInterner {
    strings: RwLock<HashMap<String, &'static str>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: RwLock::new(HashMap::new()),
        }
    }

    pub fn intern(&self, s: &str) -> &'static str {
        // Fast path: read-lock
        {
            let strings = self.strings.read().unwrap();
            if let Some(&interned) = strings.get(s) {
                return interned;
            }
        }

        // Slow path: write-lock
        let mut strings = self.strings.write().unwrap();
        if let Some(&interned) = strings.get(s) {
            return interned;
        }

        let boxed = Box::new(s.to_string());
        let leaked: &'static str = Box::leak(boxed).as_str();
        strings.insert(s.to_string(), leaked);
        leaked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_same_string() {
        let interner = StringInterner::new();
        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        assert_eq!(s1.as_ptr(), s2.as_ptr());
    }

    #[test]
    fn test_intern_different_strings() {
        let interner = StringInterner::new();
        let s1 = interner.intern("hello");
        let s2 = interner.intern("world");
        assert_ne!(s1, s2);
    }
}
