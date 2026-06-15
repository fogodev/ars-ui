//! Browser E2E harnesses for workspace components.

pub(crate) mod assertions;
pub(crate) mod axe;
pub(crate) mod browser;
pub mod desktop;
mod error;
pub(crate) mod fixtures;

/// E2E harnesses for input components.
pub mod input;

/// E2E harnesses for navigation components.
pub mod navigation;

/// E2E harnesses for utility components.
pub mod utility;

/// Browser smoke harnesses for public widgets examples.
pub mod widgets;

pub use error::Error;
