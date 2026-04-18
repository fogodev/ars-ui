// ars-collections/src/key.rs

use alloc::string::String;
use core::fmt;

/// The identifier for a node within a collection.
///
/// Keys are stable across re-renders. Framework adapters commonly derive
/// keys from item index (for static slices), from a database primary key
/// (for server data), or from a user-supplied `id` prop.
///
/// The `String` variant covers most real-world use-cases including numeric
/// IDs rendered as strings. The `Int` variant is a zero-allocation fast
/// path for purely numeric identifiers (e.g., row IDs from a `u64` database
/// primary key). The `Uuid` variant (requires the `uuid` feature) provides
/// a zero-allocation path for UUID-based identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Key {
    /// String key — the universal fallback.
    String(String),

    /// Integer key — allocation-free for numeric identifiers.
    Int(u64),

    /// UUID key — allocation-free for UUID-based identifiers.
    ///
    /// Available only when the `uuid` feature is enabled. Provides a
    /// 16-byte `Copy` key without heap allocation, compared to the 36-byte
    /// `String` representation of a UUID.
    #[cfg(feature = "uuid")]
    Uuid(uuid::Uuid),
}

/// Manual `PartialOrd` / `Ord` implementation: `Int` keys sort before `String`
/// keys so that numeric identifiers (common in database-driven collections)
/// cluster together at the front of `BTreeSet<Key>` used by `selection::State`.
/// Within each variant the natural ordering applies (`u64::cmp` for `Int`,
/// lexicographic for `String`).
///
/// When the `uuid` feature is enabled, the ordering is `Int < Uuid < String`:
/// numeric IDs first, then UUIDs (also structured identifiers), then
/// arbitrary strings.
///
/// **Note on mixed-key ordering**: When a collection contains
/// both `Key::Int` and `Key::String` keys, all `Int` keys sort before all
/// `String` keys. This is intentional for database-backed collections where
/// numeric IDs and string IDs are not intermixed. If your use case requires a
/// single unified ordering, normalize all keys to `Key::String` (e.g., via
/// `Key::str(id.to_string())`). For database-sourced numeric IDs, prefer
/// `Key::from_database_id(u64)` (alias for `Key::Int`) to make the sort
/// behavior explicit.
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Variant ordering: Int (0) < Uuid (1) < String (2).
        // Within the same variant: natural ordering.
        match (self, other) {
            (Key::Int(a), Key::Int(b)) => a.cmp(b),
            (Key::String(a), Key::String(b)) => a.cmp(b),
            (Key::Int(_), Key::String(_)) => core::cmp::Ordering::Less,
            (Key::String(_), Key::Int(_)) => core::cmp::Ordering::Greater,
            #[cfg(feature = "uuid")]
            (Key::Uuid(a), Key::Uuid(b)) => a.cmp(b),
            #[cfg(feature = "uuid")]
            (Key::Int(_), Key::Uuid(_)) => core::cmp::Ordering::Less,
            #[cfg(feature = "uuid")]
            (Key::Uuid(_), Key::Int(_)) => core::cmp::Ordering::Greater,
            #[cfg(feature = "uuid")]
            (Key::Uuid(_), Key::String(_)) => core::cmp::Ordering::Less,
            #[cfg(feature = "uuid")]
            (Key::String(_), Key::Uuid(_)) => core::cmp::Ordering::Greater,
        }
    }
}

impl Key {
    /// Construct a string key.
    #[must_use]
    pub fn str(s: impl Into<String>) -> Self {
        Key::String(s.into())
    }

    /// Construct an integer key.
    #[must_use]
    pub const fn int(n: u64) -> Self {
        Key::Int(n)
    }

    /// Construct a key from a database numeric ID.
    ///
    /// Alias for [`Key::Int`]. Exists to make the ordering behavior explicit:
    /// database ID keys sort before string keys in `BTreeSet<Key>`.
    #[must_use]
    pub const fn from_database_id(n: u64) -> Self {
        Key::Int(n)
    }

    /// Construct a UUID key.
    ///
    /// Available only when the `uuid` feature is enabled. Provides a
    /// zero-allocation key for UUID-based identifiers.
    #[cfg(feature = "uuid")]
    #[must_use]
    pub const fn uuid(id: uuid::Uuid) -> Self {
        Key::Uuid(id)
    }

    /// Parse a string as a key, attempting integer parsing first.
    ///
    /// If the string can be parsed as a `u64`, returns [`Key::Int`].
    /// Otherwise returns [`Key::String`] with the original value.
    ///
    /// To avoid ambiguity, prefer explicit constructors: [`Key::int(42)`](Key::int)
    /// or [`Key::str("42abc")`](Key::str).
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.parse::<u64>() {
            Ok(n) => Key::Int(n),
            Err(_) => Key::String(s.into()),
        }
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Key::String(s.into())
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Key::String(s)
    }
}

impl From<u64> for Key {
    fn from(n: u64) -> Self {
        Key::Int(n)
    }
}

impl From<u32> for Key {
    fn from(n: u32) -> Self {
        Key::Int(u64::from(n))
    }
}

impl From<usize> for Key {
    fn from(n: usize) -> Self {
        Key::Int(n as u64)
    }
}

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for Key {
    fn from(id: uuid::Uuid) -> Self {
        Key::Uuid(id)
    }
}

impl Default for Key {
    fn default() -> Self {
        Key::Int(0)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::String(s) => f.write_str(s),
            Key::Int(n) => write!(f, "{n}"),
            #[cfg(feature = "uuid")]
            Key::Uuid(id) => write!(f, "{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, format, string::ToString, vec, vec::Vec};

    use super::*;

    #[cfg(feature = "std")]
    fn hash_of(key: &Key) -> u64 {
        use core::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn key_int_sorts_before_string() {
        assert!(Key::Int(99) < Key::str("aaa"));
        assert!(Key::Int(0) < Key::str(""));
    }

    #[test]
    fn key_int_ordering() {
        assert!(Key::Int(1) < Key::Int(2));
        assert_eq!(Key::Int(5).cmp(&Key::Int(5)), core::cmp::Ordering::Equal);
    }

    #[test]
    fn key_string_ordering() {
        assert!(Key::str("a") < Key::str("b"));
        assert!(Key::str("abc") < Key::str("abd"));
    }

    #[test]
    fn key_from_str_ref() {
        let key = Key::from("hello");
        assert_eq!(key, Key::String("hello".into()));
    }

    #[test]
    fn key_from_string() {
        let key = Key::from(String::from("x"));
        assert_eq!(key, Key::String("x".into()));
    }

    #[test]
    fn key_from_u64() {
        let key = Key::from(42u64);
        assert_eq!(key, Key::Int(42));
    }

    #[test]
    fn key_from_u32() {
        let key = Key::from(7u32);
        assert_eq!(key, Key::Int(7));
    }

    #[test]
    fn key_from_usize() {
        let key = Key::from(3usize);
        assert_eq!(key, Key::Int(3));
    }

    #[test]
    fn key_parse_int() {
        assert_eq!(Key::parse("42"), Key::Int(42));
    }

    #[test]
    fn key_parse_string() {
        assert_eq!(Key::parse("abc"), Key::String("abc".into()));
    }

    #[test]
    fn key_parse_mixed() {
        assert_eq!(Key::parse("42abc"), Key::String("42abc".into()));
    }

    #[test]
    fn key_parse_negative() {
        // Negative numbers are not valid u64, so they become strings.
        assert_eq!(Key::parse("-1"), Key::String("-1".into()));
    }

    #[test]
    fn key_display_int() {
        assert_eq!(Key::Int(7).to_string(), "7");
    }

    #[test]
    fn key_display_string() {
        assert_eq!(Key::str("hi").to_string(), "hi");
    }

    #[test]
    fn key_default() {
        assert_eq!(Key::default(), Key::Int(0));
    }

    #[test]
    fn key_from_database_id() {
        assert_eq!(Key::from_database_id(100), Key::Int(100));
    }

    #[test]
    #[cfg(feature = "std")]
    fn key_hash_eq() {
        let a = Key::str("same");
        let b = Key::str("same");
        assert_eq!(hash_of(&a), hash_of(&b));

        let c = Key::Int(42);
        let d = Key::Int(42);
        assert_eq!(hash_of(&c), hash_of(&d));
    }

    #[test]
    fn key_btreeset_ordering() {
        let mut set = BTreeSet::new();
        set.insert(Key::str("z"));
        set.insert(Key::Int(2));
        set.insert(Key::str("a"));
        set.insert(Key::Int(1));

        let keys = set.into_iter().collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec![Key::Int(1), Key::Int(2), Key::str("a"), Key::str("z")]
        );
    }

    #[test]
    fn key_clone_and_debug() {
        let key = Key::str("test");
        let cloned = key.clone();
        assert_eq!(key, cloned);
        let debug = format!("{key:?}");
        assert!(debug.contains("String"));
    }

    #[test]
    fn key_str_constructor() {
        assert_eq!(Key::str("hello"), Key::String("hello".into()));
    }

    #[test]
    fn key_int_constructor() {
        assert_eq!(Key::int(99), Key::Int(99));
    }

    // --- Boundary / edge cases ---

    #[test]
    fn key_parse_empty_string() {
        assert_eq!(Key::parse(""), Key::String(String::new()));
    }

    #[test]
    fn key_parse_zero() {
        assert_eq!(Key::parse("0"), Key::Int(0));
    }

    #[test]
    fn key_parse_u64_max() {
        assert_eq!(Key::parse("18446744073709551615"), Key::Int(u64::MAX));
    }

    #[test]
    fn key_parse_u64_overflow() {
        // u64::MAX + 1 overflows → falls back to String
        assert_eq!(
            Key::parse("18446744073709551616"),
            Key::String("18446744073709551616".into())
        );
    }

    #[test]
    fn key_display_empty_string() {
        assert_eq!(Key::str("").to_string(), "");
    }

    #[test]
    fn key_cross_variant_inequality() {
        // Key::Int(42) and Key::str("42") are different keys — this is a
        // critical semantic guarantee for collections that mix numeric and
        // string identifiers.
        assert_ne!(Key::Int(42), Key::str("42"));
    }

    // --- UUID variant tests (feature-gated) ---

    #[cfg(feature = "uuid")]
    mod uuid_tests {
        use alloc::{collections::BTreeSet, format, vec, vec::Vec};

        use super::super::*;

        fn sample_uuid_a() -> uuid::Uuid {
            uuid::Uuid::from_bytes([
                0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44,
                0x00, 0x00,
            ])
        }

        fn sample_uuid_b() -> uuid::Uuid {
            uuid::Uuid::from_bytes([
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x4f, 0xff, 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff,
            ])
        }

        #[test]
        fn uuid_constructor() {
            let id = sample_uuid_a();
            let key = Key::uuid(id);
            assert_eq!(key, Key::Uuid(id));
        }

        #[test]
        fn uuid_from_impl() {
            let id = sample_uuid_a();
            let key = Key::from(id);
            assert_eq!(key, Key::Uuid(id));
        }

        #[test]
        fn uuid_display() {
            let id = sample_uuid_a();
            let key = Key::uuid(id);
            assert_eq!(key.to_string(), id.to_string());
        }

        #[test]
        fn uuid_debug() {
            let key = Key::uuid(sample_uuid_a());
            let debug = format!("{key:?}");
            assert!(debug.contains("Uuid"));
        }

        #[test]
        fn uuid_clone_and_eq() {
            let key = Key::uuid(sample_uuid_a());
            let cloned = key.clone();
            assert_eq!(key, cloned);
        }

        #[test]
        fn uuid_ordering_within_variant() {
            let a = Key::uuid(sample_uuid_a());
            let b = Key::uuid(sample_uuid_b());
            assert!(a < b);
        }

        #[test]
        fn uuid_sorts_after_int() {
            assert!(Key::Int(u64::MAX) < Key::uuid(sample_uuid_a()));
        }

        #[test]
        fn uuid_sorts_before_string() {
            assert!(Key::uuid(sample_uuid_b()) < Key::str(""));
        }

        #[test]
        fn uuid_cross_variant_inequality() {
            let id = sample_uuid_a();
            // Uuid and String representation of the same UUID are different keys
            assert_ne!(Key::uuid(id), Key::str(id.to_string()));
        }

        #[test]
        fn btreeset_ordering_with_uuid() {
            let id = sample_uuid_a();
            let mut set = BTreeSet::new();
            set.insert(Key::str("z"));
            set.insert(Key::uuid(id));
            set.insert(Key::Int(1));
            set.insert(Key::str("a"));
            set.insert(Key::Int(2));

            let keys = set.into_iter().collect::<Vec<_>>();
            assert_eq!(
                keys,
                vec![
                    Key::Int(1),
                    Key::Int(2),
                    Key::uuid(id),
                    Key::str("a"),
                    Key::str("z"),
                ]
            );
        }

        #[cfg(feature = "std")]
        #[test]
        fn uuid_hash_eq() {
            use core::hash::{Hash, Hasher};

            let a = Key::uuid(sample_uuid_a());
            let b = Key::uuid(sample_uuid_a());
            let mut ha = std::hash::DefaultHasher::new();
            let mut hb = std::hash::DefaultHasher::new();
            a.hash(&mut ha);
            b.hash(&mut hb);
            assert_eq!(ha.finish(), hb.finish());
        }
    }
}
