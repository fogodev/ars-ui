//! Round-trip serde tests for `ars-collections`.
//!
//! Ports the canonical `Set` round-trip test from
//! `spec/testing/13-policies.md` §252–260 and extends it with sibling
//! coverage for every other serde-derived type in the crate.
//!
//! Only compiled when the `serde` feature is enabled — the serialization
//! layer is entirely optional and should never influence the default build.

#![cfg(feature = "serde")]

use std::collections::BTreeSet;

use ars_collections::{
    AsyncLoadingState, CollectionChange, Key, Node, NodeType, SortDescriptor, SortDirection,
    selection::{Behavior, DisabledBehavior, Mode, Set, State},
};

/// Serialize `value` to JSON, deserialize it back, and return the result.
///
/// Every sibling test below runs through this helper so the JSON layer is
/// exercised consistently.
fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("must serialize");

    serde_json::from_str(&json).expect("must deserialize")
}

// ---------------------------------------------------------------------------
// Canonical test — spec/testing/13-policies.md §252–260, ported verbatim.
// ---------------------------------------------------------------------------

#[test]
fn selection_state_round_trips_via_serde() {
    let set = Set::Multiple(BTreeSet::from([Key::from("a"), Key::from("b")]));

    let json = serde_json::to_string(&set).expect("Set must serialize");

    let restored: Set = serde_json::from_str(&json).expect("Set must deserialize");

    assert_eq!(set, restored);
}

// ---------------------------------------------------------------------------
// Key
// ---------------------------------------------------------------------------

#[test]
fn key_string_round_trips() {
    let key = Key::from("alpha");

    assert_eq!(key, round_trip(&key));
}

#[test]
fn key_int_round_trips() {
    let key = Key::Int(42);

    assert_eq!(key, round_trip(&key));
}

#[cfg(feature = "uuid")]
#[test]
fn key_uuid_round_trips() {
    let key = Key::Uuid(uuid::Uuid::from_u128(
        0xdead_beef_cafe_babe_1234_5678_90ab_cdef,
    ));

    assert_eq!(key, round_trip(&key));
}

// ---------------------------------------------------------------------------
// NodeType
// ---------------------------------------------------------------------------

#[test]
fn node_type_round_trips_all_variants() {
    for nt in [
        NodeType::Item,
        NodeType::Section,
        NodeType::Header,
        NodeType::Separator,
    ] {
        assert_eq!(nt, round_trip(&nt));
    }
}

// ---------------------------------------------------------------------------
// Node<T>
// ---------------------------------------------------------------------------

#[test]
fn node_round_trips_with_value() {
    let node = Node::<String> {
        key: Key::from("leaf"),
        node_type: NodeType::Item,
        value: Some("payload".into()),
        text_value: "leaf".into(),
        level: 2,
        has_children: false,
        is_expanded: Some(false),
        parent_key: Some(Key::from("branch")),
        index: 7,
    };

    let restored: Node<String> = round_trip(&node);

    assert_eq!(node.key, restored.key);
    assert_eq!(node.node_type, restored.node_type);
    assert_eq!(node.value, restored.value);
    assert_eq!(node.text_value, restored.text_value);
    assert_eq!(node.level, restored.level);
    assert_eq!(node.has_children, restored.has_children);
    assert_eq!(node.is_expanded, restored.is_expanded);
    assert_eq!(node.parent_key, restored.parent_key);
    assert_eq!(node.index, restored.index);
}

#[test]
fn node_round_trips_without_value() {
    let node = Node::<String> {
        key: Key::from("hdr"),
        node_type: NodeType::Header,
        value: None,
        text_value: "Header".into(),
        level: 0,
        has_children: false,
        is_expanded: None,
        parent_key: None,
        index: 0,
    };

    let restored: Node<String> = round_trip(&node);

    assert_eq!(node.key, restored.key);
    assert_eq!(node.node_type, restored.node_type);
    assert!(restored.value.is_none());
    assert_eq!(node.text_value, restored.text_value);
}

// ---------------------------------------------------------------------------
// selection::Mode / Behavior / DisabledBehavior
// ---------------------------------------------------------------------------

#[test]
fn selection_mode_round_trips_all_variants() {
    for m in [Mode::None, Mode::Single, Mode::Multiple] {
        assert_eq!(m, round_trip(&m));
    }
}

#[test]
fn selection_behavior_round_trips_all_variants() {
    for b in [Behavior::Toggle, Behavior::Replace] {
        assert_eq!(b, round_trip(&b));
    }
}

#[test]
fn selection_disabled_behavior_round_trips_all_variants() {
    for b in [DisabledBehavior::Skip, DisabledBehavior::FocusOnly] {
        assert_eq!(b, round_trip(&b));
    }
}

// ---------------------------------------------------------------------------
// selection::Set — covers Empty / Single / Multiple / All
// ---------------------------------------------------------------------------

#[test]
fn selection_set_empty_round_trips() {
    let set = Set::Empty;

    assert_eq!(set, round_trip(&set));
}

#[test]
fn selection_set_single_round_trips() {
    let set = Set::Single(Key::Int(7));

    assert_eq!(set, round_trip(&set));
}

#[test]
fn selection_set_all_round_trips() {
    let set = Set::All;

    assert_eq!(set, round_trip(&set));
}

#[test]
fn selection_set_multiple_with_int_and_string_keys_round_trips() {
    // Mixed-key ordering: Int sorts before String in `BTreeSet<Key>`.
    let set = Set::Multiple(BTreeSet::from([
        Key::Int(9),
        Key::Int(3),
        Key::from("beta"),
        Key::from("alpha"),
    ]));

    assert_eq!(set, round_trip(&set));
}

// ---------------------------------------------------------------------------
// selection::State
// ---------------------------------------------------------------------------

#[test]
fn selection_state_default_round_trips() {
    let state = State::default();

    assert_eq!(state, round_trip(&state));
}

#[test]
fn selection_state_fully_populated_round_trips() {
    let mut disabled = BTreeSet::new();

    disabled.insert(Key::from("disabled"));

    let state = State {
        selected_keys: Set::Multiple(BTreeSet::from([Key::from("a"), Key::Int(2)])),
        anchor_key: Some(Key::from("a")),
        focused_key: Some(Key::Int(2)),
        disabled_keys: disabled,
        disabled_behavior: DisabledBehavior::FocusOnly,
        mode: Mode::Multiple,
        behavior: Behavior::Replace,
        selection_mode_active: true,
        max_selection: Some(5),
    };

    assert_eq!(state, round_trip(&state));
}

// ---------------------------------------------------------------------------
// SortDirection / SortDescriptor<K>
// ---------------------------------------------------------------------------

#[test]
fn sort_direction_round_trips_all_variants() {
    for d in [SortDirection::Ascending, SortDirection::Descending] {
        assert_eq!(d, round_trip(&d));
    }
}

#[test]
fn sort_descriptor_round_trips() {
    let sd = SortDescriptor {
        column: Key::from("name"),
        direction: SortDirection::Descending,
    };

    assert_eq!(sd, round_trip(&sd));
}

// ---------------------------------------------------------------------------
// AsyncLoadingState — covers every variant, including Error(String)
// ---------------------------------------------------------------------------

#[test]
fn async_loading_state_round_trips_all_variants() {
    for s in [
        AsyncLoadingState::Idle,
        AsyncLoadingState::Loading,
        AsyncLoadingState::LoadingMore,
        AsyncLoadingState::Loaded,
        AsyncLoadingState::Error("boom".into()),
    ] {
        assert_eq!(s, round_trip(&s));
    }
}

// ---------------------------------------------------------------------------
// CollectionChange<K> — covers every variant as the on-the-wire event shape
// ---------------------------------------------------------------------------

#[test]
fn collection_change_insert_round_trips() {
    let change = CollectionChange::<Key>::Insert { index: 3, count: 2 };

    assert_eq!(change, round_trip(&change));
}

#[test]
fn collection_change_remove_round_trips() {
    let change = CollectionChange::<Key>::Remove {
        keys: vec![Key::from("a"), Key::Int(1)],
    };

    assert_eq!(change, round_trip(&change));
}

#[test]
fn collection_change_move_round_trips() {
    let change = CollectionChange::<Key>::Move {
        key: Key::from("a"),
        from_index: 0,
        to_index: 4,
    };

    assert_eq!(change, round_trip(&change));
}

#[test]
fn collection_change_replace_round_trips() {
    let change = CollectionChange::<Key>::Replace {
        key: Key::from("a"),
    };

    assert_eq!(change, round_trip(&change));
}

#[test]
fn collection_change_reset_round_trips() {
    let change = CollectionChange::<Key>::Reset;

    assert_eq!(change, round_trip(&change));
}

// ---------------------------------------------------------------------------
// Regression: borrowed generic parameters must satisfy `Deserialize<'de>`.
//
// The derives on `SortDescriptor<K>`, `Node<T>`, and `CollectionChange<K>`
// must lean on serde's default bound (`Deserialize<'de>`) rather than the
// stricter `DeserializeOwned`, otherwise consumers that parameterise these
// types with borrowed keys (for example `&str`-based column IDs) cannot
// deserialize at all. This is a compile-time assertion — if the bound is
// tightened back to `DeserializeOwned`, this function fails to compile.
// ---------------------------------------------------------------------------

/// Compile-time proof that the serde derives admit borrowed generic types.
///
/// The inner `assertions` function takes a `PhantomData<&'a ()>` argument
/// so its body must type-check for an arbitrary lifetime `'a`. If the
/// derives were tightened back to `DeserializeOwned`, `&'a str` — a
/// borrowed, non-`'static` type — would fail the check and the test
/// file would no longer compile.
#[expect(
    dead_code,
    reason = "exists solely to assert serde bounds at compile time"
)]
const BORROWED_GENERIC_PARAMS_DESERIALIZE_ASSERTION: () = {
    const fn assert_deserialize<'de, T: serde::Deserialize<'de>>() {}

    const fn assertions<'a>(_marker: core::marker::PhantomData<&'a ()>) {
        assert_deserialize::<SortDescriptor<&'a str>>();
        assert_deserialize::<Node<&'a str>>();
        assert_deserialize::<CollectionChange<&'a str>>();
    }

    assertions(core::marker::PhantomData);
};
