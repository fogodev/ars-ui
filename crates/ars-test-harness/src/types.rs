//! Shared data types used by the test harness API.

/// Keyboard keys supported by the generic harness keyboard helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardKey {
    /// The Enter key.
    Enter,

    /// The Space key.
    Space,

    /// The Escape key.
    Escape,

    /// The Tab key.
    Tab,

    /// The `ArrowUp` key.
    ArrowUp,

    /// The `ArrowDown` key.
    ArrowDown,

    /// The `ArrowLeft` key.
    ArrowLeft,

    /// The `ArrowRight` key.
    ArrowRight,

    /// The Home key.
    Home,

    /// The End key.
    End,

    /// The `PageUp` key.
    PageUp,

    /// The `PageDown` key.
    PageDown,

    /// The Backspace key.
    Backspace,

    /// The Delete key.
    Delete,

    /// A printable character key.
    Char(char),
}

impl KeyboardKey {
    /// Returns the DOM `KeyboardEvent.key` value for this key.
    #[must_use]
    pub fn as_key_value(self) -> String {
        match self {
            Self::Enter => String::from("Enter"),
            Self::Space => String::from(" "),
            Self::Escape => String::from("Escape"),
            Self::Tab => String::from("Tab"),
            Self::ArrowUp => String::from("ArrowUp"),
            Self::ArrowDown => String::from("ArrowDown"),
            Self::ArrowLeft => String::from("ArrowLeft"),
            Self::ArrowRight => String::from("ArrowRight"),
            Self::Home => String::from("Home"),
            Self::End => String::from("End"),
            Self::PageUp => String::from("PageUp"),
            Self::PageDown => String::from("PageDown"),
            Self::Backspace => String::from("Backspace"),
            Self::Delete => String::from("Delete"),
            Self::Char(ch) => ch.to_string(),
        }
    }
}

/// A 2D point used by touch and pointer helpers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal coordinate in CSS pixels.
    pub x: f64,

    /// Vertical coordinate in CSS pixels.
    pub y: f64,
}

/// Creates a [`Point`] from numeric coordinates.
#[must_use]
pub fn point(x: impl Into<f64>, y: impl Into<f64>) -> Point {
    Point {
        x: x.into(),
        y: y.into(),
    }
}

/// A layout rectangle used by test assertions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left coordinate in CSS pixels.
    pub x: f64,

    /// Top coordinate in CSS pixels.
    pub y: f64,

    /// Width in CSS pixels.
    pub width: f64,

    /// Height in CSS pixels.
    pub height: f64,
}

impl Rect {
    /// Returns the right edge of the rectangle.
    #[must_use]
    pub const fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Returns the bottom edge of the rectangle.
    #[must_use]
    pub const fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Returns the left edge of the rectangle.
    #[must_use]
    pub const fn left(&self) -> f64 {
        self.x
    }

    /// Returns the top edge of the rectangle.
    #[must_use]
    pub const fn top(&self) -> f64 {
        self.y
    }
}
