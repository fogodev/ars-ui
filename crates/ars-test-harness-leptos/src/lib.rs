//! Leptos-specific test harness backend for ars-ui component testing.
//!
//! This crate will own the Leptos adapter render wrappers and concrete backend
//! implementation in issue `#181`.

use std::{any::Any, pin::Pin, time::Duration};

use ars_leptos::prelude::Locale;
use ars_test_harness::{AnyService, HarnessBackend};

/// Test harness backend that drives Leptos rendering during component tests.
#[derive(Debug, Default)]
pub struct LeptosHarnessBackend;

impl HarnessBackend for LeptosHarnessBackend {
    fn mount(
        &self,
        _container: &web_sys::HtmlElement,
        _component: Box<dyn Any>,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
        Box::pin(async { panic!("LeptosHarnessBackend::mount is implemented in issue #181") })
    }

    fn mount_with_locale(
        &self,
        _container: &web_sys::HtmlElement,
        _component: Box<dyn Any>,
        _locale: Locale,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
        Box::pin(async {
            panic!("LeptosHarnessBackend::mount_with_locale is implemented in issue #181")
        })
    }

    fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async {})
    }

    fn advance_time(&self, _duration: Duration) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async {})
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::Any,
        future::Future,
        panic::AssertUnwindSafe,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
        time::Duration,
    };

    use ars_test_harness::HarnessBackend;
    use web_sys::wasm_bindgen::{JsCast, JsValue};

    use super::LeptosHarnessBackend;

    fn panic_message(panic: &(dyn Any + Send)) -> String {
        if let Some(message) = panic.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = panic.downcast_ref::<&'static str>() {
            String::from(*message)
        } else {
            String::from("<non-string panic>")
        }
    }

    fn assert_panics_with_message<F, R>(f: F, expected: &str)
    where
        F: FnOnce() -> R,
    {
        let Err(panic) = std::panic::catch_unwind(AssertUnwindSafe(f)) else {
            panic!("operation should panic on native");
        };

        let message = panic_message(panic.as_ref());

        assert!(
            message.contains(expected),
            "expected panic containing {expected:?}, got {message:?}"
        );
    }

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn run_ready<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));

        let mut future = std::pin::pin!(future);

        let mut context = Context::from_waker(&waker);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test future unexpectedly returned Poll::Pending"),
        }
    }

    fn fake_html_element() -> web_sys::HtmlElement {
        JsValue::NULL.unchecked_into()
    }

    #[test]
    fn test_helpers_cover_remaining_paths() {
        let string_panic: Box<dyn Any + Send> = Box::new(String::from("owned"));

        let str_panic: Box<dyn Any + Send> = Box::new("borrowed");

        let other_panic: Box<dyn Any + Send> = Box::new(1usize);

        assert_eq!(panic_message(string_panic.as_ref()), "owned");
        assert_eq!(panic_message(str_panic.as_ref()), "borrowed");
        assert_eq!(panic_message(other_panic.as_ref()), "<non-string panic>");

        let helper_panic = std::panic::catch_unwind(|| assert_panics_with_message(|| (), "unused"))
            .expect_err("assert_panics_with_message should fail for non-panicking closures");

        let helper_message = panic_message(helper_panic.as_ref());

        assert!(helper_message.contains("operation should panic"));

        struct PendingFuture;

        impl Future for PendingFuture {
            type Output = ();

            fn poll(
                self: std::pin::Pin<&mut Self>,
                _context: &mut Context<'_>,
            ) -> Poll<Self::Output> {
                Poll::Pending
            }
        }

        let pending_panic = std::panic::catch_unwind(|| run_ready(PendingFuture))
            .expect_err("run_ready should panic when a future remains pending");

        let pending_message = panic_message(pending_panic.as_ref());

        assert!(pending_message.contains("Poll::Pending"));

        let waker = Waker::from(Arc::new(NoopWake));

        waker.wake_by_ref();
        waker.wake();
    }

    #[test]
    fn flush_and_advance_time_are_no_ops() {
        let backend = LeptosHarnessBackend;

        drop(backend.flush());
        drop(backend.advance_time(Duration::from_millis(1)));
    }

    #[test]
    fn mount_panics_until_issue_181_is_implemented() {
        let backend = LeptosHarnessBackend;

        let container = fake_html_element();

        assert_panics_with_message(
            || run_ready(backend.mount(&container, Box::new(()))),
            "issue #181",
        );
    }

    #[test]
    fn mount_with_locale_panics_until_issue_181_is_implemented() {
        let backend = LeptosHarnessBackend;

        let container = fake_html_element();

        assert_panics_with_message(
            || {
                run_ready(backend.mount_with_locale(
                    &container,
                    Box::new(()),
                    ars_leptos::prelude::Locale::parse("en-US").expect("locale should parse"),
                ))
            },
            "issue #181",
        );
    }
}
