//! URL validation and sanitization helpers for URL-valued HTML attributes.
//!
//! Components rendering `href`, `action`, or `formaction` attributes must
//! validate user-provided URLs before writing them to the DOM. This module
//! implements the spec allowlist for safe schemes and relative URL forms while
//! rejecting dangerous or unknown schemes such as `javascript:`.

use core::fmt::{self, Display};

/// Check whether a URL is safe for use in `href`, `action`, or `formaction`
/// attributes.
///
/// Safe URLs are limited to the explicit allowlist from the DOM utilities
/// spec: `http://`, `https://`, `mailto:`, `tel:`, `/`, `./`, `../`, `#`,
/// `?`, and relative paths with no scheme separator. Any URL containing `:`
/// that does not match the allowlist is rejected.
#[must_use]
pub fn is_safe_url(url: &str) -> bool {
    let trimmed = url.trim_start().as_bytes();

    starts_with_ignore_case(trimmed, b"http://")
        || starts_with_ignore_case(trimmed, b"https://")
        || starts_with_ignore_case(trimmed, b"mailto:")
        || starts_with_ignore_case(trimmed, b"tel:")
        || trimmed.first() == Some(&b'/')
        || trimmed.first() == Some(&b'#')
        || trimmed.first() == Some(&b'?')
        || starts_with_ignore_case(trimmed, b"./")
        || starts_with_ignore_case(trimmed, b"../")
        || !trimmed.contains(&b':')
}

/// Sanitize a URL for HTML attribute output.
///
/// Safe URLs are returned unchanged so callers can preserve the original
/// spelling. Unsafe URLs are replaced with `"#"` to prevent execution or
/// navigation through a disallowed scheme.
#[must_use]
pub fn sanitize_url(url: &str) -> &str {
    if is_safe_url(url) { url } else { "#" }
}

/// A validated URL newtype.
///
/// This type performs one-time validation at construction so components can
/// store a URL value that is already guaranteed to satisfy the DOM utilities
/// allowlist.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SafeUrl(String);

impl SafeUrl {
    /// Create a validated URL value.
    ///
    /// # Errors
    ///
    /// Returns [`UnsafeUrlError`] when the provided URL uses a disallowed or
    /// unknown scheme.
    pub fn new(url: impl Into<String>) -> Result<Self, UnsafeUrlError> {
        let url = url.into();
        if is_safe_url(&url) {
            Ok(Self(url))
        } else {
            Err(UnsafeUrlError(url))
        }
    }

    /// Borrow the validated URL string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SafeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error returned when a URL fails safe-scheme validation.
#[derive(Clone, Debug)]
pub struct UnsafeUrlError(
    /// The rejected URL string.
    pub String,
);

impl Display for UnsafeUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsafe URL scheme: {:?}", self.0)
    }
}

/// Case-insensitive prefix check without allocating.
fn starts_with_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len()
        && haystack[..needle.len()]
            .iter()
            .zip(needle)
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

#[cfg(test)]
mod tests {
    use super::{SafeUrl, UnsafeUrlError, is_safe_url, sanitize_url};

    #[test]
    fn safe_url_accepts_allowlisted_absolute_schemes() {
        assert!(is_safe_url("http://example.com"));
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("mailto:user@example.com"));
        assert!(is_safe_url("tel:+15551234567"));
    }

    #[test]
    fn safe_url_accepts_relative_urls() {
        assert!(is_safe_url(""));
        assert!(is_safe_url("/path"));
        assert!(is_safe_url("./path"));
        assert!(is_safe_url("../path"));
        assert!(is_safe_url("#anchor"));
        assert!(is_safe_url("?query=1"));
        assert!(is_safe_url("relative/path"));
    }

    #[test]
    fn safe_url_rejects_disallowed_and_unknown_schemes() {
        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url("data:text/html;base64,Zm9v"));
        assert!(!is_safe_url("vbscript:msgbox(\"x\")"));
        assert!(!is_safe_url("ftp://example.com"));
        assert!(!is_safe_url("custom:opaque"));
    }

    #[test]
    fn safe_url_rejects_relative_looking_paths_with_colons() {
        assert!(!is_safe_url("foo/bar:baz"));
        assert!(!is_safe_url("docs:readme"));
    }

    #[test]
    fn safe_url_matches_schemes_case_insensitively() {
        assert!(is_safe_url("HTTP://example.com"));
        assert!(is_safe_url("Https://example.com"));
        assert!(is_safe_url("MAILTO:user@example.com"));
        assert!(is_safe_url("TeL:+15551234567"));
        assert!(!is_safe_url("JavaScript:alert(1)"));
    }

    #[test]
    fn safe_url_trims_leading_whitespace_before_scheme_check() {
        assert!(!is_safe_url("  javascript:alert(1)"));
        assert!(is_safe_url("  https://example.com"));
    }

    #[test]
    fn sanitize_url_returns_safe_url_unchanged() {
        assert_eq!(sanitize_url("/docs"), "/docs");
    }

    #[test]
    fn sanitize_url_replaces_unsafe_url_with_hash() {
        assert_eq!(sanitize_url("javascript:alert(1)"), "#");
    }

    #[test]
    fn safe_url_new_accepts_safe_input() {
        let safe = SafeUrl::new("https://example.com").expect("safe URL should validate");

        assert_eq!(safe.as_str(), "https://example.com");
    }

    #[test]
    fn safe_url_new_rejects_unsafe_input() {
        let error = SafeUrl::new("javascript:alert(1)").expect_err("unsafe URL should be rejected");

        assert_eq!(error.0, "javascript:alert(1)");
    }

    #[test]
    fn safe_url_display_prints_validated_url() {
        let safe = SafeUrl::new("/docs").expect("safe URL should validate");

        assert_eq!(safe.to_string(), "/docs");
    }

    #[test]
    fn unsafe_url_error_display_includes_rejected_url() {
        let error = UnsafeUrlError("javascript:alert(1)".to_owned());

        assert_eq!(
            error.to_string(),
            "unsafe URL scheme: \"javascript:alert(1)\""
        );
    }
}
