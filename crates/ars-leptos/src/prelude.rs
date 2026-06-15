//! Convenience re-exports for end users of the Leptos adapter.
//!
//! A single `use ars_leptos::prelude::*;` gives application authors access to
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
//!   `ars_leptos::use_machine` for advanced users building custom machines.)
//! - **Framework internals outside `leptos::prelude`** — the adapter prelude
//!   intentionally re-exports `leptos::prelude::*` so application code can use a
//!   single `use ars_leptos::prelude::*;` import. Lower-level Leptos modules,
//!   unstable internals, and implementation-only adapter hooks still stay out of
//!   this prelude.
//!
//! # Name collisions with Leptos
//!
//! Adapter wrapper components may intentionally shadow framework component names
//! when the ars-ui wrapper is the outcome users should get by default. For
//! example, `ErrorBoundary` resolves to
//! [`crate::utility::error_boundary::ErrorBoundary`] from this prelude; users who
//! need the raw framework primitive can spell `leptos::prelude::ErrorBoundary`
//! explicitly.
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
pub use ars_forms::validation::Error as ValidationError;
pub use ars_i18n::{Direction, IntlBackend, Locale, Orientation, ResolvedDirection, Translate};
// -- Re-export Leptos prelude for end users
pub use leptos::prelude::*;

// -- Root provider --
// `ArsProvider` is the single root provider every ars-ui application wraps its
// tree with. It publishes locale, direction, color mode, disabled/read-only,
// portal/root nodes, platform effects, and style strategy via context. See
// `spec/leptos-components/utility/ars-provider.md`.
pub use crate::ArsProvider;
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
// `DismissReason`, …) and the Leptos-side wrappers (`Handle`, `Region`,
// `RegionProps`, `use_dismissable`).
pub use crate::as_child::{self, AsChildSlot};
// The `error_boundary` adapter module exposes the `ArsErrorBoundary`
// wrapper component spec'd at
// `spec/foundation/08-adapter-leptos.md` §17. End users reach it as
// `error_boundary::ArsErrorBoundary` after `use ars_leptos::prelude::*;`.
#[cfg(feature = "icu4x")]
pub use crate::utility::highlight::{self, Highlight};
// -- User-facing helpers --
pub use crate::{Translatable, root_class, t, use_id, use_number_formatter};
pub use crate::{
    input::{self, checkbox},
    navigation::{
        self,
        tabs::{self, Tabs},
    },
    utility::{
        self,
        button::{self, Button, ButtonAsChild},
        client_only::{self, ClientOnly},
        dismissable::{self, use_dismissable},
        error_boundary::{self, ErrorBoundary},
        field::{self, Field},
        fieldset::{self, Fieldset},
        form::{self, Form},
        heading::{self, Heading, HeadingLevelProvider},
        landmark::{self, Landmark},
        separator::{self, Separator, SeparatorAsChild},
        visually_hidden::{self, VisuallyHidden, VisuallyHiddenAsChild},
        z_index_allocator::{self, Context as ZIndexContext, ZIndexAllocatorProvider},
    },
};
