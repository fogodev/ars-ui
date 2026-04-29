//! Shared input mode tokens for native text controls.

/// `inputmode` values used to request mobile virtual keyboard layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Do not show a virtual keyboard automatically.
    None,

    /// Show the default text keyboard.
    Text,

    /// Show a telephone keypad.
    Tel,

    /// Show a URL-oriented keyboard.
    Url,

    /// Show an email-oriented keyboard.
    Email,

    /// Show a numeric keyboard.
    Numeric,

    /// Show a decimal keypad.
    Decimal,

    /// Show a keyboard optimized for search entry.
    Search,
}

impl InputMode {
    /// Returns the HTML `inputmode` token for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Text => "text",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::Email => "email",
            Self::Numeric => "numeric",
            Self::Decimal => "decimal",
            Self::Search => "search",
        }
    }
}
