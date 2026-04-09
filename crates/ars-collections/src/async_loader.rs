// ars-collections/src/async_loader.rs

use alloc::{string::String, vec::Vec};

/// The result of fetching a single page of items from an async data source.
///
/// Returned by [`AsyncLoader::load_page`] on success. The component machine
/// passes `items` to [`AsyncCollection::append_page`](crate::AsyncCollection::append_page)
/// and stores `next_cursor` for the next fetch.
#[derive(Clone, Debug)]
pub struct LoadResult<T> {
    /// The items returned by this page.
    pub items: Vec<T>,

    /// Opaque cursor for the next page. `None` signals that no more pages
    /// exist. The [`AsyncCollection`](crate::AsyncCollection) stores this and
    /// passes it back on the next `load_page` call.
    pub next_cursor: Option<String>,

    /// Total number of items across all pages, if the server provides it.
    /// Used to set `aria-setsize` on virtualized items before all pages
    /// have been fetched.
    pub total_count: Option<usize>,
}

/// Error type returned by async page loads.
///
/// Contains a human-readable message (for logging and retry UI) and a flag
/// indicating whether the request is worth retrying.
#[derive(Clone, Debug)]
pub struct CollectionError {
    /// A human-readable error message (not shown to end users by default;
    /// used for logging and the retry UI).
    pub message: String,

    /// Whether the caller should retry the request. Set to `false` for
    /// permanent failures (e.g., 404 Not Found).
    pub retryable: bool,
}

impl core::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Defines how to fetch a single page of data for an
/// [`AsyncCollection`](crate::AsyncCollection).
///
/// Implementations are provided by the application layer. The component
/// machine calls `load_page` inside a framework-managed async effect
/// (e.g., Leptos `create_resource`, Dioxus `use_future`).
///
/// # Examples
///
/// ```rust,ignore
/// struct UserLoader { search_query: String }
///
/// impl AsyncLoader<User> for UserLoader {
///     type Fut = Pin<Box<dyn Future<Output = Result<LoadResult<User>, CollectionError>>>>;
///
///     fn load_page(&self, cursor: Option<&str>) -> Self::Fut {
///         let query = self.search_query.clone();
///         let cursor = cursor.map(String::from);
///         Box::pin(async move {
///             let resp = api::search_users(&query, cursor.as_deref()).await?;
///             Ok(LoadResult {
///                 items: resp.users,
///                 next_cursor: resp.next_cursor,
///                 total_count: Some(resp.total),
///             })
///         })
///     }
/// }
/// ```
pub trait AsyncLoader<T> {
    /// The future type returned by `load_page`. This is an associated type
    /// rather than `async fn` to support `no_std` environments and to give
    /// callers control over boxing and pinning.
    type Fut: Future<Output = Result<LoadResult<T>, CollectionError>>;

    /// Fetch a single page of items starting from `cursor`.
    ///
    /// - `cursor` is `None` for the initial load and `Some(...)` for
    ///   subsequent pages. The value comes from [`LoadResult::next_cursor`]
    ///   returned by the previous call.
    /// - The returned future **must be cancel-safe**: dropping it before
    ///   completion cancels the in-flight request with no side effects.
    ///   This is the standard cancellation semantic — if the component
    ///   unmounts or the user navigates away, the adapter drops the future
    ///   and no further callbacks fire.
    ///
    /// # Errors
    ///
    /// Returns [`CollectionError`] when the page load fails. The
    /// `retryable` flag indicates whether the caller should offer a retry.
    fn load_page(&self, cursor: Option<&str>) -> Self::Fut;
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString, vec};
    use core::future;

    use super::*;

    // -----------------------------------------------------------
    // LoadResult tests
    // -----------------------------------------------------------

    #[test]
    fn load_result_construction() {
        let result = LoadResult {
            items: vec!["apple", "banana"],
            next_cursor: Some("cursor_2".to_string()),
            total_count: Some(100),
        };
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0], "apple");
        assert_eq!(result.next_cursor.as_deref(), Some("cursor_2"));
        assert_eq!(result.total_count, Some(100));
    }

    #[test]
    fn load_result_no_more_pages() {
        let result = LoadResult::<String> {
            items: vec![],
            next_cursor: None,
            total_count: None,
        };
        assert!(result.items.is_empty());
        assert!(result.next_cursor.is_none());
        assert!(result.total_count.is_none());
    }

    #[test]
    fn load_result_clone() {
        let original = LoadResult {
            items: vec![1, 2, 3],
            next_cursor: Some("page2".to_string()),
            total_count: Some(50),
        };
        let cloned = original.clone();
        assert_eq!(cloned.items, original.items);
        assert_eq!(cloned.next_cursor, original.next_cursor);
        assert_eq!(cloned.total_count, original.total_count);
    }

    #[test]
    fn load_result_debug() {
        let result = LoadResult {
            items: vec!["x"],
            next_cursor: None,
            total_count: None,
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("LoadResult"));
    }

    // -----------------------------------------------------------
    // CollectionError tests
    // -----------------------------------------------------------

    #[test]
    fn collection_error_display() {
        let err = CollectionError {
            message: "network timeout".to_string(),
            retryable: true,
        };
        assert_eq!(format!("{err}"), "network timeout");
    }

    #[test]
    fn collection_error_retryable_true() {
        let err = CollectionError {
            message: "server error".to_string(),
            retryable: true,
        };
        assert!(err.retryable);
    }

    #[test]
    fn collection_error_retryable_false() {
        let err = CollectionError {
            message: "not found".to_string(),
            retryable: false,
        };
        assert!(!err.retryable);
    }

    #[test]
    fn collection_error_clone() {
        let original = CollectionError {
            message: "oops".to_string(),
            retryable: true,
        };
        let cloned = original.clone();
        assert_eq!(cloned.message, "oops");
        assert!(cloned.retryable);
    }

    #[test]
    fn collection_error_debug() {
        let err = CollectionError {
            message: "fail".to_string(),
            retryable: false,
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("CollectionError"));
        assert!(debug.contains("fail"));
    }

    // -----------------------------------------------------------
    // AsyncLoader trait implementability
    // -----------------------------------------------------------

    struct MockLoader;

    impl AsyncLoader<String> for MockLoader {
        type Fut = future::Ready<Result<LoadResult<String>, CollectionError>>;

        fn load_page(&self, cursor: Option<&str>) -> Self::Fut {
            let items = if cursor.is_none() {
                vec!["first".to_string()]
            } else {
                vec!["second".to_string()]
            };
            future::ready(Ok(LoadResult {
                items,
                next_cursor: cursor.map(|_| "next".to_string()),
                total_count: None,
            }))
        }
    }

    #[test]
    fn async_loader_is_implementable() {
        let loader = MockLoader;
        // Initial load (no cursor)
        let _fut = loader.load_page(None);
        // Subsequent load (with cursor)
        let _fut = loader.load_page(Some("page2"));
    }
}
