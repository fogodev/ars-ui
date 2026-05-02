//! Toast notification component.
//!
//! See `spec/components/overlay/toast.md`. The Toast surface is split into
//! two related state machines that adapters compose:
//!
//! * [`single`] — per-toast lifecycle (`Visible → Paused → Dismissing →
//!   Dismissed`), auto-dismiss countdown intent, swipe gesture state, and
//!   ARIA / data attribute production for the per-toast anatomy parts.
//! * [`manager`] — coordinates queue admission, max-visible cap,
//!   deduplication, pause-all/resume-all, and the §4.2 announcement queue
//!   so adapters can drain the polite/assertive live regions in priority +
//!   FIFO order at 500 ms intervals. Also owns the canonical
//!   [`manager::region_attrs`] helper so the `aria-live` shells stamp a
//!   single `data-ars-scope="toast"`.
//!
//! Portal placement, real `set_timeout` wiring, swipe pointer-capture,
//! focus/hover DOM listeners, page-visibility wiring, the `Toaster`
//! imperative handle, and the promise-toast spawn glue stay in adapters.
//! The agnostic core only emits typed [`single::Effect`] /
//! [`manager::Effect`] intents and exposes [`single::Api`] /
//! [`manager::Api`] for ARIA / data attribute production, plus the
//! [`manager::Toaster`] zero-sized config-builder factory.

pub mod manager;
pub mod single;

pub use manager::{
    AnnouncePriority, Config, DefaultDurations, EdgeOffsets, EntryStage, Placement, Promise,
    RegionPart, SwipeAxis, ToastContent, ToastEntry, Toaster, region_attrs,
};
pub use single::{
    Api, Context, DEFAULT_SWIPE_THRESHOLD, Effect, Event, Kind, Machine, Messages, Part, Props,
    State,
};
