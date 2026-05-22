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
pub use ars_collections::{Key, TabKey};
// `ColorMode` is a configuration enum that end users pass to `ArsProvider`
// via the `color_mode` prop; it belongs with the configuration types.
pub use ars_core::{
    ColorMode, I18nRegistries, MessageFn, MessagesRegistry, SafeUrl, UnsafeUrlError,
};
pub use ars_i18n::{Direction, IntlBackend, Locale, Orientation, ResolvedDirection, Translate};

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
pub use crate::as_child;
pub use crate::navigation::{self, tabs};
// The `error_boundary` adapter module exposes the `ArsErrorBoundary`
// wrapper component spec'd at
// `spec/foundation/09-adapter-dioxus.md` §21. End users reach it as
// `error_boundary::ArsErrorBoundary` after `use ars_dioxus::prelude::*;`.
pub use crate::utility::{
    self, button, client_only, dismissable, error_boundary, heading, highlight, landmark,
    separator, visually_hidden, z_index_allocator,
};
// -- Root provider --
// `ArsProvider` is the single root provider every ars-ui application wraps its
// tree with. It publishes locale, direction, color mode, disabled/read-only,
// portal/root nodes, platform effects, and style strategy via context. The
// explicit `ArsProviderProps` struct is re-exported so end users can build
// props value at call sites without importing it separately. See
// `spec/dioxus-components/utility/ars-provider.md`.
pub use crate::{ArsProvider, ArsProviderProps};
// -- User-facing helpers --
pub use crate::{Translatable, t, use_number_formatter};
