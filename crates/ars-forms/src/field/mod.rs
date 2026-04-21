//! Field runtime state, inherited context, descriptors, and component machine.

pub mod component;
mod context;
pub mod descriptors;
mod state;
mod value;
mod value_ext;

pub use context::Context;
pub use descriptors::{Descriptors, InputAria};
pub use state::State;
pub use value::{FileRef, Value};
pub use value_ext::{CheckboxExt, SelectionExt, ValueExt};
