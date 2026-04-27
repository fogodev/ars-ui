//! Convenience re-exports for end users of the Dioxus adapter.
//!
//! A single `use ars_dioxus::prelude::*;` gives application authors access to
//! the components, user-facing traits, and configuration types they need —
//! without hunting through individual `ars_*` crates.
//!
//! # What belongs in the prelude
//!
//! Only items that **end users** interact with directly:
//!
//! 1. **Component modules** — as components land (e.g., `button`, `dialog`,
//!    `select`), their public module paths are re-exported here so users can
//!    write `button::Props`, `dialog::Machine`, etc.
//!
//! 2. **User-facing traits** — traits that end users call on component outputs
//!    (e.g., `Translate` from `ars-i18n` for localised strings). Re-exporting
//!    the trait avoids forcing users to add a direct dependency on the
//!    subsystem crate just to call a method.
//!
//! 3. **Configuration types** — types that appear in component props or that
//!    users pass to configure behaviour (e.g., `Locale`, `Direction`,
//!    `Orientation`, `Selection`).
//!
//! # What does NOT belong in the prelude
//!
//! - **Core engine internals** — `Machine`, `Service`, `ConnectApi`,
//!   `ComponentPart`, `AttrMap`, `Bindable`, `TransitionPlan`, `PendingEffect`.
//!   These are implementation details used by component authors *inside* the
//!   adapter crate, not by end users consuming components.
//! - **Accessibility primitives** — `AriaRole`, `AriaAttribute`, `ComponentIds`.
//!   Wired internally by each component; end users never construct these.
//! - **Interaction primitives** — `merge_attrs`, `PressState`, `FocusState`.
//!   Internal to component implementations.
//! - **Adapter hooks** — `use_machine`, `UseMachineReturn`, `EphemeralRef`.
//!   Used by component modules inside the adapter crate, not by end users
//!   who consume the ready-made components. (Still publicly accessible via
//!   `ars_dioxus::use_machine` for advanced users building custom machines.)
//! - **Framework re-exports** — `dioxus::prelude::*` should be imported
//!   separately; we do not re-export Dioxus types to avoid version coupling.
//!
//! # Growth policy
//!
//! When adding a new item, ask: "Does an end user writing `<Button>` or
//! `<Dialog>` in their app need this?" If yes, add it. If only component
//! implementors inside this crate need it, keep it as a regular import.

// -- User-facing traits --
pub use ars_i18n::{Direction, Locale, Orientation, ResolvedDirection, Translate};

// -- Component modules --
//
// Consumers reach component types via the module qualifier
// (e.g. `dismissable::Props`, `dismissable::Region`, `dismissable::Handle`,
// `dismissable::DismissReason`, `dismissable::use_dismissable`); never via
// flattened aliases such as `DismissableProps` or `DismissableRegion`.
//
// The adapter `dismissable` module re-exports the agnostic
// `ars_components::utility::dismissable::*` surface, so this single
// re-export covers both the framework-agnostic types (`Props`, `Messages`,
// `DismissReason`, …) and the Dioxus-side wrappers (`Handle`, `Region`,
// `RegionProps`, `use_dismissable`).
pub use crate::dismissable;
// -- User-facing helpers --
pub use crate::{t, use_number_formatter};
