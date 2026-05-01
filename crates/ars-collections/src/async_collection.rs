// ars-collections/src/async_collection.rs

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug};

use crate::{Collection, Key, Node, StaticCollection};

/// The current loading phase of an async collection or page.
///
/// Tracks whether a collection is idle, actively loading its initial or
/// subsequent data, fully loaded, or in an error state requiring retry.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AsyncLoadingState {
    /// No load has been initiated yet.
    #[default]
    Idle,

    /// The initial load is in progress. The collection is empty.
    Loading,

    /// Additional pages are being fetched. The collection already has items.
    LoadingMore,

    /// Data is fully loaded. No more pages.
    Loaded,

    /// A load failed. The error message is surfaced for display or retry.
    Error(String),
}

impl AsyncLoadingState {
    /// Returns `true` when a load is in flight (initial or subsequent page).
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        matches!(self, Self::Loading | Self::LoadingMore)
    }

    /// Returns `true` when the collection is in an error state.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Returns the error message if in the `Error` state, `None` otherwise.
    #[must_use]
    pub const fn error_message(&self) -> Option<&str> {
        if let Self::Error(msg) = self {
            Some(msg.as_str())
        } else {
            None
        }
    }
}

/// A collection that grows over time as pages are fetched.
///
/// The component machine drives loading: when the sentinel element (the last
/// rendered item or a dedicated loading indicator) becomes visible, the
/// machine emits a `LoadMore` event, which triggers the async fetch effect.
/// When the fetch completes, the machine merges new items via
/// [`append_page`](Self::append_page).
///
/// `AsyncCollection<T>` wraps a [`StaticCollection<T>`] (the items loaded so
/// far) with cursor-based pagination metadata and loading state. It is not
/// itself reactive — the component's machine context holds it in a `Signal`
/// or similar framework primitive.
///
/// All mutation methods follow the immutable-update pattern: they take `&self`
/// and return a new `Self`, leaving the original unchanged.
pub struct AsyncCollection<T> {
    /// Items loaded so far.
    inner: StaticCollection<T>,

    /// Opaque cursor for the next page request. `None` means either the
    /// collection has not started loading or all pages are exhausted.
    pub next_cursor: Option<String>,

    /// Whether more pages remain to be fetched.
    pub has_more: bool,

    /// Current loading phase.
    pub loading_state: AsyncLoadingState,

    /// Total item count if known from the server (e.g., from a `total`
    /// field in the API response). `None` when unknown.
    pub total_count: Option<usize>,
}

impl<T: Clone> AsyncCollection<T> {
    /// Create an empty async collection ready for its first load.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: StaticCollection::default(),
            next_cursor: None,
            has_more: true,
            loading_state: AsyncLoadingState::Idle,
            total_count: None,
        }
    }

    /// Transition to the loading state before a fetch begins.
    ///
    /// Returns `Loading` when the collection is empty (initial fetch) or
    /// `LoadingMore` when items already exist (subsequent page).
    #[must_use]
    pub fn begin_load(&self) -> Self {
        let state = if self.inner.is_empty() {
            AsyncLoadingState::Loading
        } else {
            AsyncLoadingState::LoadingMore
        };

        Self {
            loading_state: state,
            ..self.clone_meta()
        }
    }

    /// Append a new page of items, updating cursor and `has_more`.
    ///
    /// Merges the existing items with `new_items` into a fresh
    /// [`StaticCollection`], sets `loading_state` to `Loaded`, and updates
    /// `has_more` based on whether `next_cursor` is `Some`.
    #[must_use]
    pub fn append_page(
        &self,
        new_items: Vec<(Key, String, T)>,
        next_cursor: Option<String>,
    ) -> Self {
        let has_more = next_cursor.is_some();

        // Merge existing items with the new page.
        let mut merged = self
            .inner
            .nodes()
            .filter_map(|n| {
                n.value
                    .as_ref()
                    .map(|v| (n.key.clone(), n.text_value.clone(), v.clone()))
            })
            .collect::<Vec<_>>();

        merged.extend(new_items);

        Self {
            inner: merged.into(),
            next_cursor,
            has_more,
            loading_state: AsyncLoadingState::Loaded,
            total_count: self.total_count,
        }
    }

    /// Record a load error.
    ///
    /// Transitions `loading_state` to `Error` while preserving all loaded
    /// items and pagination metadata.
    #[must_use]
    pub fn set_error(&self, message: impl Into<String>) -> Self {
        Self {
            loading_state: AsyncLoadingState::Error(message.into()),
            ..self.clone_meta()
        }
    }

    /// Clone all fields. Used by the manual `Clone` impl and mutation methods.
    fn clone_meta(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            next_cursor: self.next_cursor.clone(),
            has_more: self.has_more,
            loading_state: self.loading_state.clone(),
            total_count: self.total_count,
        }
    }
}

impl<T: Clone> Default for AsyncCollection<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Delegate Collection<T> to the inner StaticCollection.
impl<T: Clone> Collection<T> for AsyncCollection<T> {
    fn size(&self) -> usize {
        self.inner.size()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        self.inner.get(key)
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.inner.get_by_index(index)
    }

    fn first_key(&self) -> Option<&Key> {
        self.inner.first_key()
    }

    fn last_key(&self) -> Option<&Key> {
        self.inner.last_key()
    }

    fn key_after(&self, key: &Key) -> Option<&Key> {
        self.inner.key_after(key)
    }

    fn key_before(&self, key: &Key) -> Option<&Key> {
        self.inner.key_before(key)
    }

    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
        self.inner.key_after_no_wrap(key)
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        self.inner.key_before_no_wrap(key)
    }

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a,
    {
        self.inner.keys()
    }

    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.inner.nodes()
    }

    fn children_of<'a>(&'a self, parent_key: &Key) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.inner.children_of(parent_key)
    }
}

impl<T: Clone> Clone for AsyncCollection<T> {
    fn clone(&self) -> Self {
        self.clone_meta()
    }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints size, loading state,
/// and `has_more` since the payload `T` is opaque to the machine layer.
impl<T: Clone> Debug for AsyncCollection<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncCollection")
            .field("size", &self.inner.size())
            .field("loading_state", &self.loading_state)
            .field("has_more", &self.has_more)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString, vec, vec::Vec};

    use super::*;

    // ---------------------------------------------------------------
    // AsyncLoadingState tests
    // ---------------------------------------------------------------

    #[test]
    fn default_is_idle() {
        assert_eq!(AsyncLoadingState::default(), AsyncLoadingState::Idle);
    }

    #[test]
    fn is_loading_true_for_loading_and_loading_more() {
        assert!(AsyncLoadingState::Loading.is_loading());
        assert!(AsyncLoadingState::LoadingMore.is_loading());
    }

    #[test]
    fn is_loading_false_for_others() {
        assert!(!AsyncLoadingState::Idle.is_loading());
        assert!(!AsyncLoadingState::Loaded.is_loading());
        assert!(!AsyncLoadingState::Error("x".to_string()).is_loading());
    }

    #[test]
    fn is_error_true() {
        assert!(AsyncLoadingState::Error("fail".to_string()).is_error());
    }

    #[test]
    fn is_error_false_for_others() {
        assert!(!AsyncLoadingState::Idle.is_error());
        assert!(!AsyncLoadingState::Loading.is_error());
        assert!(!AsyncLoadingState::LoadingMore.is_error());
        assert!(!AsyncLoadingState::Loaded.is_error());
    }

    #[test]
    fn error_message_some() {
        let state = AsyncLoadingState::Error("oops".to_string());

        assert_eq!(state.error_message(), Some("oops"));
    }

    #[test]
    fn error_message_none_for_non_error() {
        assert_eq!(AsyncLoadingState::Idle.error_message(), None);
        assert_eq!(AsyncLoadingState::Loading.error_message(), None);
        assert_eq!(AsyncLoadingState::LoadingMore.error_message(), None);
        assert_eq!(AsyncLoadingState::Loaded.error_message(), None);
    }

    #[test]
    fn loading_state_clone_and_eq() {
        let original = AsyncLoadingState::Error("network".to_string());

        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn loading_state_debug() {
        let state = AsyncLoadingState::LoadingMore;

        let debug = format!("{state:?}");

        assert!(debug.contains("LoadingMore"));
    }

    // ---------------------------------------------------------------
    // AsyncCollection — construction
    // ---------------------------------------------------------------

    #[test]
    fn new_is_empty_with_has_more() {
        let c = AsyncCollection::<String>::new();

        assert_eq!(c.size(), 0);
        assert!(c.is_empty());
        assert!(c.has_more);
        assert_eq!(c.loading_state, AsyncLoadingState::Idle);
        assert!(c.next_cursor.is_none());
        assert!(c.total_count.is_none());
    }

    #[test]
    fn default_equals_new() {
        let a = AsyncCollection::<String>::new();

        let b = AsyncCollection::<String>::default();

        assert_eq!(a.size(), b.size());
        assert_eq!(a.has_more, b.has_more);
        assert_eq!(a.loading_state, b.loading_state);
        assert_eq!(a.next_cursor, b.next_cursor);
        assert_eq!(a.total_count, b.total_count);
    }

    // ---------------------------------------------------------------
    // AsyncCollection::begin_load
    // ---------------------------------------------------------------

    #[test]
    fn begin_load_empty_sets_loading() {
        let c = AsyncCollection::<String>::new();

        let loading = c.begin_load();

        assert_eq!(loading.loading_state, AsyncLoadingState::Loading);
    }

    #[test]
    fn begin_load_with_items_sets_loading_more() {
        let c = AsyncCollection::<&str>::new();

        let loaded = c.append_page(
            vec![(Key::int(1), "Apple".to_string(), "a")],
            Some("cursor_2".to_string()),
        );

        let loading = loaded.begin_load();

        assert_eq!(loading.loading_state, AsyncLoadingState::LoadingMore);
    }

    // ---------------------------------------------------------------
    // AsyncCollection::append_page
    // ---------------------------------------------------------------

    #[test]
    fn append_page_adds_items_and_sets_loaded() {
        let c = AsyncCollection::<&str>::new();

        let page1 = c.append_page(
            vec![
                (Key::int(1), "Apple".to_string(), "a"),
                (Key::int(2), "Banana".to_string(), "b"),
                (Key::int(3), "Cherry".to_string(), "c"),
            ],
            Some("cursor_2".to_string()),
        );

        assert_eq!(page1.size(), 3);
        assert_eq!(page1.loading_state, AsyncLoadingState::Loaded);
        assert!(page1.get(&Key::int(1)).is_some());
        assert!(page1.get(&Key::int(2)).is_some());
        assert!(page1.get(&Key::int(3)).is_some());
    }

    #[test]
    fn append_page_merges_with_existing() {
        let c = AsyncCollection::<&str>::new();

        let page1 = c.append_page(
            vec![
                (Key::int(1), "Apple".to_string(), "a"),
                (Key::int(2), "Banana".to_string(), "b"),
            ],
            Some("cursor_2".to_string()),
        );

        let page2 = page1.append_page(
            vec![
                (Key::int(3), "Cherry".to_string(), "c"),
                (Key::int(4), "Date".to_string(), "d"),
            ],
            None,
        );

        assert_eq!(page2.size(), 4);
        // All items accessible
        assert!(page2.get(&Key::int(1)).is_some());
        assert!(page2.get(&Key::int(4)).is_some());
    }

    #[test]
    fn append_page_cursor_controls_has_more() {
        let c = AsyncCollection::<&str>::new();

        // With cursor → has_more = true
        let with_cursor = c.append_page(
            vec![(Key::int(1), "A".to_string(), "a")],
            Some("next".to_string()),
        );

        assert!(with_cursor.has_more);
        assert_eq!(with_cursor.next_cursor.as_deref(), Some("next"));

        // Without cursor → has_more = false
        let final_page = with_cursor.append_page(vec![(Key::int(2), "B".to_string(), "b")], None);

        assert!(!final_page.has_more);
        assert!(final_page.next_cursor.is_none());
    }

    #[test]
    fn append_page_preserves_total_count() {
        let mut c = AsyncCollection::<&str>::new();

        c.total_count = Some(100);

        let page = c.append_page(
            vec![(Key::int(1), "A".to_string(), "a")],
            Some("cursor".to_string()),
        );

        assert_eq!(page.total_count, Some(100));
    }

    // ---------------------------------------------------------------
    // AsyncCollection::set_error
    // ---------------------------------------------------------------

    #[test]
    fn set_error_transitions_and_preserves_items() {
        let c = AsyncCollection::<&str>::new();

        let loaded = c.append_page(
            vec![(Key::int(1), "Apple".to_string(), "a")],
            Some("cursor_2".to_string()),
        );

        let errored = loaded.set_error("network timeout");

        assert_eq!(
            errored.loading_state,
            AsyncLoadingState::Error("network timeout".to_string())
        );
        assert_eq!(
            errored.loading_state.error_message(),
            Some("network timeout")
        );

        // Items preserved
        assert_eq!(errored.size(), 1);
        assert!(errored.get(&Key::int(1)).is_some());

        // Pagination metadata preserved
        assert!(errored.has_more);
        assert_eq!(errored.next_cursor.as_deref(), Some("cursor_2"));
    }

    // ---------------------------------------------------------------
    // Collection trait delegation
    // ---------------------------------------------------------------

    fn loaded_collection() -> AsyncCollection<&'static str> {
        let c = AsyncCollection::new();

        c.append_page(
            vec![
                (Key::int(1), "Apple".to_string(), "a"),
                (Key::int(2), "Banana".to_string(), "b"),
                (Key::int(3), "Cherry".to_string(), "c"),
            ],
            None,
        )
    }

    #[test]
    fn collection_delegation_size_get_navigation() {
        let c = loaded_collection();

        assert_eq!(c.size(), 3);
        assert!(!c.is_empty());
        assert!(c.contains_key(&Key::int(2)));
        assert!(!c.contains_key(&Key::int(99)));

        // get + get_by_index
        let node = c.get(&Key::int(2)).expect("key 2 exists");

        assert_eq!(node.text_value, "Banana");

        let node = c.get_by_index(0).expect("index 0");

        assert_eq!(node.key, Key::int(1));

        // Boundary navigation
        assert_eq!(c.first_key(), Some(&Key::int(1)));
        assert_eq!(c.last_key(), Some(&Key::int(3)));

        // Sequential navigation
        assert_eq!(c.key_after(&Key::int(1)), Some(&Key::int(2)));
        assert_eq!(c.key_before(&Key::int(3)), Some(&Key::int(2)));

        // Wrapping
        assert_eq!(c.key_after(&Key::int(3)), Some(&Key::int(1)));
        assert_eq!(c.key_before(&Key::int(1)), Some(&Key::int(3)));

        // No-wrap
        assert_eq!(c.key_after_no_wrap(&Key::int(1)), Some(&Key::int(2)));
        assert_eq!(c.key_before_no_wrap(&Key::int(3)), Some(&Key::int(2)));
        assert_eq!(c.key_after_no_wrap(&Key::int(3)), None);
        assert_eq!(c.key_before_no_wrap(&Key::int(1)), None);

        // text_value_of (default impl on Collection)
        assert_eq!(c.text_value_of(&Key::int(1)), Some("Apple"));
    }

    #[test]
    fn collection_delegation_keys_iterator() {
        let c = loaded_collection();

        let keys = c.keys().collect::<Vec<_>>();

        assert_eq!(keys, vec![&Key::int(1), &Key::int(2), &Key::int(3)]);

        let nodes = c.nodes().map(|n| n.text_value.as_str()).collect::<Vec<_>>();

        assert_eq!(nodes, vec!["Apple", "Banana", "Cherry"]);

        let item_keys = c.item_keys().collect::<Vec<_>>();

        assert_eq!(item_keys, vec![&Key::int(1), &Key::int(2), &Key::int(3)]);
    }

    #[test]
    fn collection_delegation_children_of_empty() {
        let c = loaded_collection();

        let children = c.children_of(&Key::str("nonexistent")).collect::<Vec<_>>();

        assert!(children.is_empty());
    }

    // ---------------------------------------------------------------
    // Manual trait impls
    // ---------------------------------------------------------------

    #[test]
    fn clone_produces_equal_state() {
        let c = AsyncCollection::<&str>::new();

        let loaded = c.append_page(
            vec![(Key::int(1), "Apple".to_string(), "a")],
            Some("cursor".to_string()),
        );

        let cloned = loaded.clone();

        assert_eq!(cloned.size(), loaded.size());
        assert_eq!(cloned.has_more, loaded.has_more);
        assert_eq!(cloned.loading_state, loaded.loading_state);
        assert_eq!(cloned.next_cursor, loaded.next_cursor);
        assert_eq!(cloned.total_count, loaded.total_count);
    }

    #[test]
    fn debug_contains_type_name() {
        let c = AsyncCollection::<String>::new();

        let debug = format!("{c:?}");

        assert!(debug.contains("AsyncCollection"));
        assert!(debug.contains("size"));
        assert!(debug.contains("loading_state"));
        assert!(debug.contains("has_more"));
    }

    // ---------------------------------------------------------------
    // Edge cases — semantic coverage gaps
    // ---------------------------------------------------------------

    #[test]
    fn begin_load_after_error_retries_same_cursor() {
        // Spec §5.2.2: retry calls begin_load() after Error, re-using
        // the same cursor that failed.
        let c = AsyncCollection::<&str>::new();

        let page1 = c.append_page(
            vec![(Key::int(1), "Apple".to_string(), "a")],
            Some("cursor_2".to_string()),
        );

        let errored = page1.set_error("network timeout");
        assert!(errored.loading_state.is_error());

        // Retry: begin_load from error state with existing items
        let retrying = errored.begin_load();

        assert_eq!(retrying.loading_state, AsyncLoadingState::LoadingMore);

        // Cursor preserved for retry
        assert_eq!(retrying.next_cursor.as_deref(), Some("cursor_2"));

        // Items still present
        assert_eq!(retrying.size(), 1);
    }

    #[test]
    fn append_empty_page_with_cursor() {
        // Server returns zero items on a page but still has a next cursor
        // (e.g., all items on this page were filtered out server-side).
        let c = AsyncCollection::<&str>::new();

        let page1 = c.append_page(
            vec![(Key::int(1), "Apple".to_string(), "a")],
            Some("cursor_2".to_string()),
        );

        let page2 = page1.append_page(vec![], Some("cursor_3".to_string()));

        // Original item preserved, no new items added
        assert_eq!(page2.size(), 1);
        assert!(page2.get(&Key::int(1)).is_some());

        // Still has more pages
        assert!(page2.has_more);
        assert_eq!(page2.next_cursor.as_deref(), Some("cursor_3"));
        assert_eq!(page2.loading_state, AsyncLoadingState::Loaded);
    }

    #[test]
    fn append_empty_page_final() {
        // Server returns zero items and no cursor (all done, last page empty).
        let c = AsyncCollection::<&str>::new();

        let page1 = c.append_page(
            vec![(Key::int(1), "Apple".to_string(), "a")],
            Some("cursor_2".to_string()),
        );

        let final_page = page1.append_page(vec![], None);

        assert_eq!(final_page.size(), 1);
        assert!(!final_page.has_more);
        assert!(final_page.next_cursor.is_none());
    }

    #[test]
    fn total_count_survives_multiple_appends() {
        let mut c = AsyncCollection::<&str>::new();

        c.total_count = Some(500);

        let page1 = c.append_page(
            vec![(Key::int(1), "A".to_string(), "a")],
            Some("c2".to_string()),
        );

        assert_eq!(page1.total_count, Some(500));

        let page2 = page1.append_page(
            vec![(Key::int(2), "B".to_string(), "b")],
            Some("c3".to_string()),
        );

        assert_eq!(page2.total_count, Some(500));

        let errored = page2.set_error("fail");

        assert_eq!(errored.total_count, Some(500));

        let retrying = errored.begin_load();

        assert_eq!(retrying.total_count, Some(500));
    }
}
