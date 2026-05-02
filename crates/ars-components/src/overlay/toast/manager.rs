//! Toast provider/manager state machine.
//!
//! Coordinates a fleet of [`single`](super::single) toasts: queue admission
//! when the visible count would exceed `max_visible`, deduplication of
//! identical kind+title+description (across both visible *and* queued
//! entries — see `spec/components/overlay/toast.md` §2.3), pause-all and
//! resume-all hooks for hover and page-visibility, dismiss-all, stacking
//! metadata derived from the placement, and the §4.2 announcement-
//! coordination queue (priority + FIFO + 500 ms drain).
//!
//! The agnostic core never reads `performance.now()`. Adapters drive the
//! 500 ms drain on their own clock and dispatch
//! [`Event::DrainAnnouncement`] with the current timestamp baked in; the
//! machine pops the head entry off
//! [`ManagerContext::announcement_queue`] (assertive first, polite second)
//! and emits the matching announcement intent through [`Effect`].

use alloc::{
    collections::VecDeque,
    string::{String, ToString as _},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentMessages, ComponentPart, ConnectApi, Env, HtmlAttr,
    Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::Hotkey;

use super::single::Kind;

// ────────────────────────────────────────────────────────────────────
// Effect
// ────────────────────────────────────────────────────────────────────

/// Typed identifier for every named effect intent the manager emits.
///
/// Adapters dispatch on `effect.name` exhaustively, so new variants surface
/// at compile time everywhere the manager is consumed. The variant names
/// themselves are the contract — there is no parallel kebab-case wire form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter inserts a polite-priority announcement into the polite
    /// `aria-live` region. Emitted by [`Event::DrainAnnouncement`] when the
    /// head of the queue is `AnnouncePriority::Polite`, and by `Add` /
    /// `Update` for newly admitted polite toasts.
    AnnouncePolite,

    /// Adapter inserts an assertive-priority announcement into the
    /// assertive `aria-live` region. Emitted by [`Event::DrainAnnouncement`]
    /// when the head of the queue is `AnnouncePriority::Assertive`, and by
    /// `Add` / `Update` for newly admitted assertive toasts.
    AnnounceAssertive,

    /// Adapter starts (or restarts) its 500 ms heartbeat that re-emits
    /// [`Event::DrainAnnouncement`] until [`ManagerContext::announcement_queue`]
    /// is empty. Emitted whenever a new entry is pushed onto an empty
    /// announcement queue.
    ScheduleAnnouncement,

    /// Adapter forwards `Event::Pause` to every visible per-toast machine.
    /// Emitted on `Event::PauseAll` and on `Event::SetVisibility(false)`.
    PauseAllTimers,

    /// Adapter forwards `Event::Resume` to every visible per-toast machine.
    /// Emitted on `Event::ResumeAll` and on `Event::SetVisibility(true)` if
    /// the manager was previously paused.
    ResumeAllTimers,

    /// Adapter forwards `Event::Dismiss` to every visible per-toast machine.
    /// Emitted on `Event::DismissAll`.
    DismissAllToasts,
}

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States for the toast manager.
///
/// Toast managers do not have an open/closed lifecycle the way overlays do,
/// but the [`State`] enum still exists so the machine satisfies the
/// `ars_core::Machine` contract and so adapters can observe whether the
/// global timer pause is in effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// Default state — visible toasts run their auto-dismiss timers.
    #[default]
    Active,

    /// All visible toast timers are paused (hover over the region or page
    /// idle). New toasts are still admitted but their `single::Machine`
    /// adapter wiring sees the global pause.
    Paused,
}

// ────────────────────────────────────────────────────────────────────
// Placement
// ────────────────────────────────────────────────────────────────────

/// Where toasts appear on screen.
///
/// The first six variants are RTL-aware (`Start`/`End` resolves to `Left`/
/// `Right` based on the document direction). The last four are physical
/// variants for callers that explicitly want left/right positioning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Placement {
    /// Top edge, inline-start corner (RTL-aware).
    TopStart,

    /// Top edge, horizontally centered.
    TopCenter,

    /// Top edge, inline-end corner (RTL-aware).
    TopEnd,

    /// Bottom edge, inline-start corner (RTL-aware).
    BottomStart,

    /// Bottom edge, horizontally centered.
    BottomCenter,

    /// Bottom edge, inline-end corner (RTL-aware). Default.
    #[default]
    BottomEnd,

    /// Top edge, physical-left corner.
    TopLeft,

    /// Top edge, physical-right corner.
    TopRight,

    /// Bottom edge, physical-left corner.
    BottomLeft,

    /// Bottom edge, physical-right corner.
    BottomRight,
}

impl Placement {
    /// Returns the wire token used for `data-ars-placement`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopStart => "top-start",
            Self::TopCenter => "top-center",
            Self::TopEnd => "top-end",
            Self::BottomStart => "bottom-start",
            Self::BottomCenter => "bottom-center",
            Self::BottomEnd => "bottom-end",
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
        }
    }

    /// Returns the swipe axis the placement implies. Center placements
    /// swipe vertically; edge placements swipe horizontally per
    /// `spec/components/overlay/toast.md` §8.3.
    #[must_use]
    pub const fn swipe_axis(self) -> SwipeAxis {
        match self {
            Self::TopCenter | Self::BottomCenter => SwipeAxis::Vertical,
            _ => SwipeAxis::Horizontal,
        }
    }
}

/// Axis along which the per-toast swipe gesture is measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeAxis {
    /// Horizontal swipe (left/right placements).
    Horizontal,

    /// Vertical swipe (center placements).
    Vertical,
}

// ────────────────────────────────────────────────────────────────────
// Announcement priority
// ────────────────────────────────────────────────────────────────────

/// Live-region urgency for a queued announcement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnnouncePriority {
    /// Polite live region — `Info`, `Success`, `Loading`.
    Polite,

    /// Assertive live region — `Warning`, `Error`. Drained before any
    /// polite entry within the same heartbeat.
    Assertive,
}

// ────────────────────────────────────────────────────────────────────
// PauseReasons
// ────────────────────────────────────────────────────────────────────

/// Independent sources that can pause every visible toast's auto-dismiss
/// timer.
///
/// `State::Paused` is reached whenever **any** reason is active and is
/// only cleared when **every** reason becomes inactive. This means a tab
/// hide → tab show cycle that overlaps with a still-active hover/focus
/// pause leaves the manager paused, instead of incorrectly resuming
/// timers while the user is reading or interacting (spec §1.5 / §2 —
/// pause sources are orthogonal).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PauseReasons {
    /// Pause requested by [`Event::PauseAll`] (typically a hover/focus
    /// over the region container, or programmatic pause from the
    /// adapter). Cleared by [`Event::ResumeAll`].
    pub interaction: bool,

    /// Pause requested by [`Event::SetVisibility`]`(false)` — the page
    /// became hidden via the Page Visibility API. Cleared by
    /// [`Event::SetVisibility`]`(true)`.
    pub visibility: bool,
}

impl PauseReasons {
    /// Returns `true` if at least one pause source is currently active.
    /// The manager is in [`State::Paused`] iff this returns `true`.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.interaction || self.visibility
    }
}

// ────────────────────────────────────────────────────────────────────
// EdgeOffsets / DefaultDurations / Config / ToastEntry
// ────────────────────────────────────────────────────────────────────

/// Safe-area insets from viewport edges in pixels.
///
/// Prevents toasts from overlapping browser chrome or system UI. Adapters
/// translate these into CSS custom properties on the region container.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeOffsets {
    /// Distance from the top edge.
    pub top: f64,

    /// Distance from the right edge.
    pub right: f64,

    /// Distance from the bottom edge.
    pub bottom: f64,

    /// Distance from the left edge.
    pub left: f64,
}

/// Default auto-dismiss durations per toast kind.
///
/// Used by [`Machine`] as a fallback when a per-toast [`Config::duration`]
/// is `None`. `loading` is `None` by default so promise-style toasts persist
/// until explicitly updated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefaultDurations {
    /// Default duration for `Kind::Info`. Defaults to 5 s.
    pub info: Duration,

    /// Default duration for `Kind::Success`. Defaults to 5 s.
    pub success: Duration,

    /// Default duration for `Kind::Warning`. Defaults to 5 s.
    pub warning: Duration,

    /// Default duration for `Kind::Error`. Defaults to 8 s — error toasts
    /// get more reading time per Ark UI's defaults.
    pub error: Duration,

    /// Default duration for `Kind::Loading`. `None` means persistent (the
    /// toast stays on-screen until updated).
    pub loading: Option<Duration>,
}

impl Default for DefaultDurations {
    fn default() -> Self {
        Self {
            info: Duration::from_secs(5),
            success: Duration::from_secs(5),
            warning: Duration::from_secs(5),
            error: Duration::from_secs(8),
            loading: None,
        }
    }
}

impl DefaultDurations {
    /// Returns the configured default duration for `kind`.
    #[must_use]
    pub const fn for_kind(&self, kind: Kind) -> Option<Duration> {
        match kind {
            Kind::Info => Some(self.info),
            Kind::Success => Some(self.success),
            Kind::Warning => Some(self.warning),
            Kind::Error => Some(self.error),
            Kind::Loading => self.loading,
        }
    }
}

/// Per-toast configuration accepted by [`Event::Add`] / [`Event::Update`].
#[derive(Clone, Debug)]
pub struct Config {
    /// Optional explicit id. When `None`, the manager generates one of the
    /// form `toast-<n>` from a monotonic counter.
    pub id: Option<String>,

    /// Toast title.
    pub title: Option<String>,

    /// Toast description.
    pub description: Option<String>,

    /// Toast urgency / appearance category.
    pub kind: Kind,

    /// Auto-dismiss duration. `None` falls back to the manager's
    /// [`DefaultDurations::for_kind`] lookup.
    pub duration: Option<Duration>,

    /// Whether the toast can be dismissed via its close button.
    pub dismissible: bool,

    /// When `true`, an `Add` carrying the same `kind` + `title` +
    /// `description` as a live or queued entry resets the existing toast
    /// (via [`Event::Update`] for visible matches, or in-place replacement
    /// for queued matches) instead of stacking another duplicate.
    pub deduplicate: bool,

    /// Optional callback fired when the per-toast pause state changes.
    /// Stored on [`ToastEntry`] so adapters can re-invoke it.
    pub on_pause_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.title == other.title
            && self.description == other.description
            && self.kind == other.kind
            && self.duration == other.duration
            && self.dismissible == other.dismissible
            && self.deduplicate == other.deduplicate
            // Callback compares by Arc pointer identity.
            && match (&self.on_pause_change, &other.on_pause_change) {
                (None, None) => true,
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            id: None,
            title: None,
            description: None,
            kind: Kind::Info,
            duration: None,
            dismissible: true,
            deduplicate: false,
            on_pause_change: None,
        }
    }
}

impl Config {
    /// Builds a [`Config`] with the supplied title and kind.
    #[must_use]
    pub fn new(kind: Kind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: Some(title.into()),
            ..Self::default()
        }
    }

    /// Sets [`description`](Self::description).
    #[must_use]
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Sets [`id`](Self::id), the explicit toast id used to override the
    /// auto-generated one.
    #[must_use]
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    /// Sets [`duration`](Self::duration).
    #[must_use]
    pub const fn duration(mut self, value: Option<Duration>) -> Self {
        self.duration = value;
        self
    }

    /// Sets [`dismissible`](Self::dismissible).
    #[must_use]
    pub const fn dismissible(mut self, value: bool) -> Self {
        self.dismissible = value;
        self
    }

    /// Sets [`deduplicate`](Self::deduplicate).
    #[must_use]
    pub const fn deduplicate(mut self, value: bool) -> Self {
        self.deduplicate = value;
        self
    }
}

/// Lifecycle stage of an entry tracked by the manager.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryStage {
    /// Entry is on-screen and counts against `max_visible`.
    Visible,

    /// Entry's per-toast machine reached `Dismissing`. The manager keeps it
    /// in [`ManagerContext::toasts`] for `remove_delay` so the exit
    /// animation can run before the row is removed.
    Dismissing,
}

/// One toast tracked by the manager.
#[derive(Clone, Debug, PartialEq)]
pub struct ToastEntry {
    /// Stable id used to address the toast in `Update`/`Remove`.
    pub id: String,

    /// User-supplied configuration.
    pub config: Config,

    /// Lifecycle stage observed by the manager.
    pub stage: EntryStage,
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// Events accepted by the toast manager.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Add a new toast. The manager admits it if `toasts.len() <
    /// max_visible`; otherwise the config is pushed onto the queue and
    /// promoted later when an entry is removed.
    Add(Config),

    /// Update an existing toast's content/kind. Resets the announcement
    /// queue entry so screen readers re-announce the new content.
    Update(String, Config),

    /// Remove a toast by id. The manager moves the entry to
    /// `EntryStage::Dismissing` so adapters can run the exit animation,
    /// then drops it after `remove_delay` via [`Event::HideQueueAdvance`].
    Remove(String),

    /// Pause the auto-dismiss timer for every visible toast. Emits
    /// [`Effect::PauseAllTimers`].
    PauseAll,

    /// Resume the auto-dismiss timer for every visible toast. Emits
    /// [`Effect::ResumeAllTimers`].
    ResumeAll,

    /// Dismiss every visible toast. Emits [`Effect::DismissAllToasts`].
    DismissAll,

    /// Adapter heartbeat — drains the next announcement entry if at least
    /// 500 ms have elapsed since the last drain. Carries the current
    /// adapter-clock timestamp (ms) so the gate is enforced atomically.
    DrainAnnouncement {
        /// Current adapter-clock timestamp in milliseconds.
        now_ms: u64,
    },

    /// Per-toast machine reported `State::Dismissed` (or its `remove_delay`
    /// elapsed). The manager removes the entry and promotes the next
    /// queued config if any.
    HideQueueAdvance(String),

    /// Page Visibility API report. `false` pauses all timers; `true`
    /// resumes them if the manager was previously paused due to visibility.
    SetVisibility(bool),

    /// Reapply context-relevant fields from the latest [`Props`].
    /// Auto-emitted by [`Machine::on_props_changed`] whenever the consumer
    /// passes a new `Props` value to `Service::set_props`. Fields that
    /// are derived per-toast at admission time
    /// ([`DefaultDurations`](Self::Add)'s fallback lookup,
    /// [`deduplicate_all`](Props::deduplicate_all)) take effect on the
    /// **next** `Add`, not retroactively.
    SyncProps,
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Runtime context for the toast manager.
#[derive(Clone, Debug, PartialEq)]
pub struct ManagerContext {
    /// Toast entries currently tracked by the manager (visible + dismissing).
    pub toasts: Vec<ToastEntry>,

    /// Configs awaiting admission because the visible count is at
    /// `max_visible`. Promoted in FIFO order on `Remove` /
    /// `HideQueueAdvance`.
    pub queued: VecDeque<Config>,

    /// Pending announcements (toast id + priority). Drained by
    /// [`Event::DrainAnnouncement`] in priority + FIFO order.
    pub announcement_queue: VecDeque<(String, AnnouncePriority)>,

    /// Adapter clock timestamp (ms) of the most recent announcement drain.
    /// Updated through [`Event::DrainAnnouncement`] so the next drain can
    /// enforce the §4.2 500 ms gap.
    pub last_announcement_at: Option<u64>,

    /// Maximum number of simultaneously visible toasts.
    pub max_visible: usize,

    /// Where toasts appear on screen.
    pub placement: Placement,

    /// Pixel gap between visible toasts.
    pub gap: f64,

    /// Delay between the per-toast machine reaching `Dismissing` and the
    /// manager forgetting it. Allows exit animations to complete.
    pub remove_delay: Duration,

    /// Default auto-dismiss durations per kind.
    pub default_durations: DefaultDurations,

    /// When `true`, every `Add` defaults to `deduplicate = true` regardless
    /// of the per-config flag.
    pub deduplicate_all: bool,

    /// Safe-area insets passed through to adapters via the region container.
    pub offsets: EdgeOffsets,

    /// When `true`, toasts visually overlap (stacked-card mode).
    pub overlap: bool,

    /// Whether all timers are currently paused (mirrors `State::Paused`).
    ///
    /// Equivalent to `pause_reasons.any()` — kept as a denormalized
    /// flag so adapters can read it without going through `pause_reasons`.
    pub paused_all: bool,

    /// Per-source pause flags. The manager remains in [`State::Paused`]
    /// while any reason is active, so a tab-hide / tab-show cycle no
    /// longer cancels an in-progress hover/focus pause.
    pub pause_reasons: PauseReasons,

    /// Resolved locale used for the region label and per-toast messages.
    pub locale: Locale,

    /// Resolved manager-level messages.
    pub messages: Messages,

    /// Monotonic counter used to build auto-generated toast ids
    /// (`toast-1`, `toast-2`, …). Consumers MUST NOT depend on the format —
    /// the field is `pub(crate)` so that adapters cannot accidentally
    /// observe or mutate it.
    pub(crate) next_id: u64,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Immutable configuration for the toast manager.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id (hydration-stable).
    pub id: String,

    /// Where toasts appear on screen. Default: `BottomEnd`.
    pub placement: Placement,

    /// Maximum number of simultaneously visible toasts. Default: 5.
    pub max_visible: usize,

    /// Pixel gap between visible toasts. Default: 16.0.
    pub gap: f64,

    /// Delay before removing a dismissed toast. Default: 200 ms.
    pub remove_delay: Duration,

    /// Default auto-dismiss durations per kind.
    pub default_durations: DefaultDurations,

    /// When `true`, every `Add` defaults to deduplicate. Default: false.
    pub deduplicate_all: bool,

    /// Whether the region pauses all timers on hover. Default: true.
    pub pause_on_hover: bool,

    /// Whether to pause timers when the page becomes idle (Page Visibility
    /// API). Default: true.
    pub pause_on_page_idle: bool,

    /// Safe-area insets from viewport edges (pixels). Default: zeroed.
    pub offsets: EdgeOffsets,

    /// When `true`, toasts visually overlap (stacked-card mode). Default: false.
    pub overlap: bool,

    /// Optional keyboard shortcut for moving focus into the region.
    /// Adapters install a global `keydown` listener and call
    /// [`Hotkey::matches`] from it; on a match the adapter moves focus
    /// to the rendered region container.
    pub hotkey: Option<Hotkey>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            placement: Placement::default(),
            max_visible: 5,
            gap: 16.0,
            remove_delay: Duration::from_millis(200),
            default_durations: DefaultDurations::default(),
            deduplicate_all: false,
            pause_on_hover: true,
            pause_on_page_idle: true,
            offsets: EdgeOffsets::default(),
            overlap: false,
            hotkey: None,
        }
    }
}

impl Props {
    /// Returns manager props with documented default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`placement`](Self::placement).
    #[must_use]
    pub const fn placement(mut self, value: Placement) -> Self {
        self.placement = value;
        self
    }

    /// Sets [`max_visible`](Self::max_visible). Clamped to a minimum of 1.
    #[must_use]
    pub const fn max_visible(mut self, value: usize) -> Self {
        self.max_visible = if value == 0 { 1 } else { value };
        self
    }

    /// Sets [`gap`](Self::gap).
    #[must_use]
    pub const fn gap(mut self, value: f64) -> Self {
        self.gap = value;
        self
    }

    /// Sets [`remove_delay`](Self::remove_delay).
    #[must_use]
    pub const fn remove_delay(mut self, value: Duration) -> Self {
        self.remove_delay = value;
        self
    }

    /// Sets [`default_durations`](Self::default_durations).
    #[must_use]
    pub const fn default_durations(mut self, value: DefaultDurations) -> Self {
        self.default_durations = value;
        self
    }

    /// Sets [`deduplicate_all`](Self::deduplicate_all).
    #[must_use]
    pub const fn deduplicate_all(mut self, value: bool) -> Self {
        self.deduplicate_all = value;
        self
    }

    /// Sets [`pause_on_hover`](Self::pause_on_hover).
    #[must_use]
    pub const fn pause_on_hover(mut self, value: bool) -> Self {
        self.pause_on_hover = value;
        self
    }

    /// Sets [`pause_on_page_idle`](Self::pause_on_page_idle).
    #[must_use]
    pub const fn pause_on_page_idle(mut self, value: bool) -> Self {
        self.pause_on_page_idle = value;
        self
    }

    /// Sets [`offsets`](Self::offsets).
    #[must_use]
    pub const fn offsets(mut self, value: EdgeOffsets) -> Self {
        self.offsets = value;
        self
    }

    /// Sets [`overlap`](Self::overlap).
    #[must_use]
    pub const fn overlap(mut self, value: bool) -> Self {
        self.overlap = value;
        self
    }

    /// Sets [`hotkey`](Self::hotkey).
    #[must_use]
    pub const fn hotkey(mut self, value: Hotkey) -> Self {
        self.hotkey = Some(value);
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable strings exposed by the toast manager.
///
/// The manager owns `region_label` because the `aria-live` region shells
/// belong to the manager's lifetime, not any individual toast. Per-toast
/// labels (e.g. dismiss-button) live on
/// [`super::single::Messages`](super::single::Messages).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the toast region landmark. Defaults to
    /// `"Notifications"`.
    pub region_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            region_label: MessageFn::static_str("Notifications"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// Structural parts exposed by the manager connect API.
///
/// `Root` carries the manager-shell scope (`toast-provider`) so CSS
/// authors can target the placement-positioned outer container distinctly
/// from the `aria-live` region shells, which share the per-toast scope
/// (`toast`) — see [`Api::region_attrs`].
#[derive(ComponentPart)]
#[scope = "toast-provider"]
pub enum Part {
    /// Outer container. Adapters render the placement-positioned shell here.
    Root,
}

/// Structural part identifier shared between the manager (which renders
/// the live-region shells) and the per-toast surface (which lives inside
/// them). Both helpers stamp `data-ars-scope="toast"` so styling and
/// query selectors target a single canonical scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionPart {
    /// Polite live region — `Info`, `Success`, `Loading`.
    Polite,

    /// Assertive live region — `Warning`, `Error`.
    Assertive,
}

impl RegionPart {
    const fn is_assertive(self) -> bool {
        matches!(self, Self::Assertive)
    }
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the toast manager.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = ManagerContext;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Active,
            ManagerContext {
                toasts: Vec::new(),
                queued: VecDeque::new(),
                announcement_queue: VecDeque::new(),
                last_announcement_at: None,
                max_visible: props.max_visible.max(1),
                placement: props.placement,
                gap: props.gap,
                remove_delay: props.remove_delay,
                default_durations: props.default_durations,
                deduplicate_all: props.deduplicate_all,
                offsets: props.offsets,
                overlap: props.overlap,
                paused_all: false,
                pause_reasons: PauseReasons::default(),
                locale: env.locale.clone(),
                messages: messages.clone(),
                next_id: 0,
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Add(config) => Some(plan_add(ctx, config.clone())),

            Event::Update(id, config) => Some(plan_update(ctx, id, config.clone())),

            Event::Remove(id) => Some(plan_remove(ctx, id.clone())),

            Event::HideQueueAdvance(id) => Some(plan_hide_queue_advance(ctx, id.clone())),

            Event::PauseAll => match state {
                // Active → Paused: arm the interaction reason, emit the
                // pause-all effect so adapters forward it to per-toast
                // machines.
                State::Active => Some(
                    TransitionPlan::to(State::Paused)
                        .apply(|ctx: &mut ManagerContext| {
                            ctx.pause_reasons.interaction = true;
                            ctx.paused_all = true;
                        })
                        .with_effect(PendingEffect::named(Effect::PauseAllTimers)),
                ),
                // Already paused for some other reason (most likely
                // visibility). Just record the additional reason — no
                // state change, no extra `PauseAllTimers` effect (timers
                // are already paused). The flag matters on resume so a
                // later `SetVisibility(true)` does not unpause us while
                // interaction pause is still in force.
                State::Paused => {
                    if ctx.pause_reasons.interaction {
                        None
                    } else {
                        Some(TransitionPlan::context_only(|ctx: &mut ManagerContext| {
                            ctx.pause_reasons.interaction = true;
                        }))
                    }
                }
            },

            Event::ResumeAll => match state {
                State::Paused => {
                    if !ctx.pause_reasons.interaction {
                        // Interaction reason already cleared (or never
                        // set); nothing to do.
                        None
                    } else if ctx.pause_reasons.visibility {
                        // Clear interaction but stay paused — page is
                        // still hidden, timers must not restart.
                        Some(TransitionPlan::context_only(|ctx: &mut ManagerContext| {
                            ctx.pause_reasons.interaction = false;
                        }))
                    } else {
                        // Last reason removed — fully resume.
                        Some(
                            TransitionPlan::to(State::Active)
                                .apply(|ctx: &mut ManagerContext| {
                                    ctx.pause_reasons.interaction = false;
                                    ctx.paused_all = false;
                                })
                                .with_effect(PendingEffect::named(Effect::ResumeAllTimers)),
                        )
                    }
                }
                State::Active => None,
            },

            Event::DismissAll => Some(
                TransitionPlan::context_only(|ctx: &mut ManagerContext| {
                    ctx.queued.clear();

                    for entry in &mut ctx.toasts {
                        entry.stage = EntryStage::Dismissing;
                    }

                    // The user dismissed everything. Pending announcements
                    // for content that's about to disappear would surface
                    // as stale screen-reader output, so drop them.
                    ctx.announcement_queue.clear();
                })
                .with_effect(PendingEffect::named(Effect::DismissAllToasts)),
            ),

            Event::DrainAnnouncement { now_ms } => Some(plan_drain_announcement(ctx, *now_ms)),

            Event::SetVisibility(visible) => match (*visible, state) {
                // Page hidden, manager active → pause for visibility.
                (false, State::Active) => Some(
                    TransitionPlan::to(State::Paused)
                        .apply(|ctx: &mut ManagerContext| {
                            ctx.pause_reasons.visibility = true;
                            ctx.paused_all = true;
                        })
                        .with_effect(PendingEffect::named(Effect::PauseAllTimers)),
                ),
                // Page hidden while interaction-paused: arm the
                // visibility reason without re-emitting `PauseAllTimers`
                // (timers are already paused). The flag matters when
                // interaction pause later clears: we must stay paused.
                (false, State::Paused) => {
                    if ctx.pause_reasons.visibility {
                        None
                    } else {
                        Some(TransitionPlan::context_only(|ctx: &mut ManagerContext| {
                            ctx.pause_reasons.visibility = true;
                        }))
                    }
                }
                // Page visible again.
                (true, State::Paused) => {
                    if !ctx.pause_reasons.visibility {
                        // Visibility reason already cleared (or never
                        // set, e.g. tests sending `SetVisibility(true)`
                        // while only interaction-paused) — nothing to do.
                        None
                    } else if ctx.pause_reasons.interaction {
                        // Clear visibility but stay paused — user is
                        // still hovering / focused; timers must not
                        // restart yet.
                        Some(TransitionPlan::context_only(|ctx: &mut ManagerContext| {
                            ctx.pause_reasons.visibility = false;
                        }))
                    } else {
                        // Last reason removed — fully resume.
                        Some(
                            TransitionPlan::to(State::Active)
                                .apply(|ctx: &mut ManagerContext| {
                                    ctx.pause_reasons.visibility = false;
                                    ctx.paused_all = false;
                                })
                                .with_effect(PendingEffect::named(Effect::ResumeAllTimers)),
                        )
                    }
                }
                // Already in the requested visibility (e.g.
                // `SetVisibility(true)` while Active) — noop.
                _ => None,
            },

            Event::SyncProps => {
                // Mirror context-backed fields from the latest props so
                // runtime updates to placement, max_visible, gap, etc.
                // take effect without recreating the service. `next_id`,
                // `paused_all`, the toast/queue/announcement collections,
                // and clock state are NOT touched — they are runtime
                // bookkeeping, not configuration.
                //
                // Clamping `max_visible` to 1 mirrors the `Props::max_visible`
                // builder; never let it drop to zero at runtime.
                let placement = props.placement;
                let max_visible = props.max_visible.max(1);
                let gap = props.gap;
                let remove_delay = props.remove_delay;
                let default_durations = props.default_durations;
                let deduplicate_all = props.deduplicate_all;
                let offsets = props.offsets;
                let overlap = props.overlap;

                Some(TransitionPlan::context_only(
                    move |ctx: &mut ManagerContext| {
                        ctx.placement = placement;
                        ctx.max_visible = max_visible;
                        ctx.gap = gap;
                        ctx.remove_delay = remove_delay;
                        ctx.default_durations = default_durations;
                        ctx.deduplicate_all = deduplicate_all;
                        ctx.offsets = offsets;
                        ctx.overlap = overlap;
                    },
                ))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // Manager id is asserted unchangeable to keep ARIA wiring stable.
        assert_eq!(
            old.id, new.id,
            "Toast manager id cannot change after initialization"
        );

        if context_relevant_props_changed(old, new) {
            alloc::vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Returns `true` when any context-backed prop differs between `old` and
/// `new`. Used by [`Machine::on_props_changed`] to decide whether to
/// emit [`Event::SyncProps`].
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.placement != new.placement
        || old.max_visible != new.max_visible
        || old.gap != new.gap
        || old.remove_delay != new.remove_delay
        || old.default_durations != new.default_durations
        || old.deduplicate_all != new.deduplicate_all
        || old.offsets != new.offsets
        || old.overlap != new.overlap
}

// ────────────────────────────────────────────────────────────────────
// Plan helpers
// ────────────────────────────────────────────────────────────────────

fn plan_add(ctx: &ManagerContext, config: Config) -> TransitionPlan<Machine> {
    // Explicit-id collision: if the caller supplied an `id` that's already
    // in flight (visible / dismissing or queued), route the Add through
    // `plan_update` / `plan_replace_queued_by_id` so addressing-by-id
    // remains unambiguous. Two entries sharing an id would silently
    // break `Update(id)` / `Remove(id)` first-match lookups.
    if let Some(explicit_id) = config.id.clone()
        && let Some(state) = locate_existing_id(ctx, &explicit_id)
    {
        return match state {
            ExistingIdLocation::Tracked => plan_update(ctx, &explicit_id, config),
            ExistingIdLocation::Queued => {
                plan_replace_queued_by_id(&explicit_id, config, ctx.default_durations)
            }
        };
    }

    if let Some(existing_id) = find_visible_dedup_match(ctx, &config) {
        return plan_update(ctx, &existing_id, config);
    }

    if find_queued_dedup_match_index(ctx, &config).is_some() {
        return plan_replace_queued(config, ctx.default_durations);
    }

    let visible_count = ctx
        .toasts
        .iter()
        .filter(|entry| entry.stage == EntryStage::Visible)
        .count();

    let admit = visible_count < ctx.max_visible;

    let max_visible = ctx.max_visible;
    let deduplicate_all = ctx.deduplicate_all;
    let default_durations = ctx.default_durations;

    let (new_id, kind) = (resolve_or_generate_id(ctx, &config), config.kind);

    let mut plan = TransitionPlan::context_only(move |ctx: &mut ManagerContext| {
        let mut config = config;

        if deduplicate_all {
            config.deduplicate = true;
        }

        if config.duration.is_none() {
            config.duration = default_durations.for_kind(kind);
        }

        let id = config.id.clone().unwrap_or_else(|| new_id.clone());

        config.id = Some(id.clone());

        if admit {
            ctx.toasts.push(ToastEntry {
                id: id.clone(),
                config,
                stage: EntryStage::Visible,
            });

            ctx.announcement_queue
                .push_back((id, kind.announce_priority()));
        } else {
            ctx.queued.push_back(config);
        }

        // Always advance the id counter so the next auto-generated id is
        // unique even if the caller supplied an explicit id this time.
        ctx.next_id = ctx.next_id.saturating_add(1);

        // Cap the queue at `max_visible * 32` to avoid unbounded growth
        // in pathological loops.
        let queue_cap = max_visible.saturating_mul(32);
        while ctx.queued.len() > queue_cap {
            ctx.queued.pop_front();
        }
    });

    if admit {
        // Announcements go through the queue exclusively — the actual
        // `AnnouncePolite` / `AnnounceAssertive` effect fires from
        // `plan_drain_announcement` once the adapter heartbeat ticks.
        // Emitting the effect here as well would announce the same toast
        // twice (immediately and again on the first drain).
        if ctx.announcement_queue.is_empty() {
            plan = plan.with_effect(PendingEffect::named(Effect::ScheduleAnnouncement));
        }

        // `kind` is only consumed inside the apply closure for the
        // priority lookup — we no longer use it for an effect here.
        let _ = kind;
    }

    plan
}

/// Where an existing entry with a given id currently lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExistingIdLocation {
    /// In `ctx.toasts` — either `Visible` or `Dismissing`.
    Tracked,

    /// In `ctx.queued`, awaiting admission.
    Queued,
}

/// Locates an existing entry that already carries the supplied id. Used by
/// `plan_add` to detect explicit-id collisions before pushing a new entry.
fn locate_existing_id(ctx: &ManagerContext, id: &str) -> Option<ExistingIdLocation> {
    if ctx.toasts.iter().any(|entry| entry.id == id) {
        Some(ExistingIdLocation::Tracked)
    } else if ctx.queued.iter().any(|cfg| cfg.id.as_deref() == Some(id)) {
        Some(ExistingIdLocation::Queued)
    } else {
        None
    }
}

/// Fills `config.duration` from `default_durations.for_kind(kind)` when
/// the caller left it unset. Called on every plan that writes a `Config`
/// into `ctx.toasts` / `ctx.queued` so a user who builds a config with
/// `Config::new(...)` (which leaves `duration: None`) doesn't silently
/// flip an auto-dismissing toast into a persistent one.
const fn fill_default_duration(config: &mut Config, default_durations: DefaultDurations) {
    if config.duration.is_none() {
        config.duration = default_durations.for_kind(config.kind);
    }
}

/// Replaces a queued entry by its explicit id. Used by `plan_add` when the
/// caller supplies an id that's already on a queued slot.
fn plan_replace_queued_by_id(
    id: &str,
    config: Config,
    default_durations: DefaultDurations,
) -> TransitionPlan<Machine> {
    let id_for_apply = id.to_string();
    TransitionPlan::context_only(move |ctx: &mut ManagerContext| {
        if let Some(slot) = ctx
            .queued
            .iter_mut()
            .find(|cfg| cfg.id.as_deref() == Some(id_for_apply.as_str()))
        {
            // Preserve the matched id verbatim so adapter-side bookkeeping
            // (timers, callbacks already wired against this slot) stays
            // intact.
            let mut config = config;

            config.id = Some(id_for_apply.clone());

            fill_default_duration(&mut config, default_durations);

            *slot = config;
        }
    })
}

fn plan_replace_queued(
    config: Config,
    default_durations: DefaultDurations,
) -> TransitionPlan<Machine> {
    // Replace the matching queued entry in place. No announcement effect
    // is scheduled because the queued config has not been admitted yet.
    TransitionPlan::context_only(move |ctx: &mut ManagerContext| {
        if let Some(index) = find_queued_dedup_match_index(ctx, &config) {
            // Preserve the existing queued slot's id so adapter-side
            // bookkeeping (timers, callbacks) doesn't drift.
            let existing_id = ctx.queued[index].id.clone();

            let mut config = config;

            config.id = existing_id.or(config.id);

            fill_default_duration(&mut config, default_durations);

            ctx.queued[index] = config;
        }
    })
}

fn plan_update(ctx: &ManagerContext, id: &str, config: Config) -> TransitionPlan<Machine> {
    let kind = config.kind;
    let default_durations = ctx.default_durations;

    let mut plan = TransitionPlan::context_only({
        let id = id.to_string();
        move |ctx: &mut ManagerContext| {
            let mut config = config;

            config.id = Some(id.clone());

            // Apply the same `DefaultDurations` fallback `plan_add`
            // applies. Without this, an `Update(id, Config::new(...))`
            // (or a dedup-routed `Add` with `Config::new`, which leaves
            // `duration: None`) would silently flip an auto-dismissing
            // toast into a persistent one.
            fill_default_duration(&mut config, default_durations);

            if let Some(entry) = ctx.toasts.iter_mut().find(|entry| entry.id == id) {
                // Preserve the entry's current `stage`. If the entry was
                // already `Dismissing` (e.g. an `Update` arrives in the
                // window between `Remove(id)` and `HideQueueAdvance(id)`),
                // forcing it back to `Visible` here would only revive it
                // briefly before the pending advance removes it again,
                // producing a confusing "popped back, then disappeared"
                // animation. Updating the content is fine — it's just a
                // configuration replacement — but the lifecycle stage is
                // owned by the dismiss flow.
                entry.config = config;
            } else {
                // Update on a queued entry: replace by id if present.
                for queued in &mut ctx.queued {
                    if queued.id.as_deref() == Some(id.as_str()) {
                        *queued = config;

                        return;
                    }
                }
            }
        }
    });

    // Only announce when the matched entry is `Visible`. A `Dismissing`
    // entry is animating out, so re-announcing the updated content would
    // be jarring — and the toast may disappear before the screen reader
    // finishes speaking. Queued entries also do not announce until they
    // are actually admitted.
    let entry_visible = ctx
        .toasts
        .iter()
        .any(|entry| entry.id == id && entry.stage == EntryStage::Visible);

    if entry_visible {
        // As in `plan_add`, announcements always flow through the queue
        // — the announce effect fires from `plan_drain_announcement`,
        // never directly from `Update`.
        if ctx.announcement_queue.is_empty() {
            plan = plan.with_effect(PendingEffect::named(Effect::ScheduleAnnouncement));
        }

        // Push the new announcement onto the queue at the tail so the
        // drain respects FIFO ordering relative to other adds.
        let priority = kind.announce_priority();

        plan = plan.apply({
            let id = id.to_string();
            move |ctx: &mut ManagerContext| {
                ctx.announcement_queue.push_back((id, priority));
            }
        });
    }

    plan
}

fn plan_remove(_ctx: &ManagerContext, id: String) -> TransitionPlan<Machine> {
    TransitionPlan::context_only(move |ctx: &mut ManagerContext| {
        if let Some(entry) = ctx.toasts.iter_mut().find(|entry| entry.id == id) {
            entry.stage = EntryStage::Dismissing;
        } else {
            // Removing a queued (not-yet-admitted) toast should drop it
            // outright — the user asked it never to surface.
            ctx.queued
                .retain(|cfg| cfg.id.as_deref() != Some(id.as_str()));
        }

        // Drop any pending announcement for this toast — the user
        // dismissed it, so a stale "X appeared" announcement after the
        // fact would just confuse the screen-reader user.
        ctx.announcement_queue
            .retain(|(queued_id, _)| queued_id != &id);
    })
}

fn plan_hide_queue_advance(ctx: &ManagerContext, id: String) -> TransitionPlan<Machine> {
    // Predict whether a queued config will be promoted; used to decide
    // whether to schedule an announcement effect upfront.
    let visible_after_remove = ctx
        .toasts
        .iter()
        .filter(|entry| entry.stage == EntryStage::Visible && entry.id != id)
        .count();

    let max_visible = ctx.max_visible;

    let promote_kind = if visible_after_remove < max_visible {
        ctx.queued.front().map(|cfg| cfg.kind)
    } else {
        None
    };

    let mut plan = TransitionPlan::context_only(move |ctx: &mut ManagerContext| {
        ctx.toasts.retain(|entry| entry.id != id);

        // Drop any pending announcement for the toast we just removed —
        // it disappeared from the visible list, so its queued
        // announcement (if any) is stale.
        ctx.announcement_queue
            .retain(|(queued_id, _)| queued_id != &id);

        let visible_count = ctx
            .toasts
            .iter()
            .filter(|entry| entry.stage == EntryStage::Visible)
            .count();

        if visible_count < max_visible
            && let Some(next) = ctx.queued.pop_front()
        {
            let kind = next.kind;

            // Invariant: every code path that pushes a `Config` into
            // `ctx.queued` (`plan_add`, `plan_update`,
            // `plan_replace_queued`, `plan_replace_queued_by_id`)
            // applies `fill_default_duration` first, so a queued config
            // either has `duration: Some(_)` or matches a kind whose
            // default is `None` (currently only `Kind::Loading`). No
            // promotion-time fallback is needed.

            // Invariant: `plan_add` and `plan_update` always set
            // `config.id = Some(...)` before pushing to `ctx.queued`, so
            // the queued config always carries an id.
            let promoted_id = next
                .id
                .clone()
                .expect("queued configs always carry an id assigned at admission");

            ctx.toasts.push(ToastEntry {
                id: promoted_id.clone(),
                config: next,
                stage: EntryStage::Visible,
            });

            ctx.announcement_queue
                .push_back((promoted_id, kind.announce_priority()));
        }
    });

    if let Some(_kind) = promote_kind {
        // The promoted toast is enqueued inside the apply closure above;
        // the announce effect itself fires from `plan_drain_announcement`.
        // Only schedule the heartbeat when the queue *was* empty — once
        // the apply closure runs, it will no longer be empty for the
        // duration of the drain cycle.
        if ctx.announcement_queue.is_empty() {
            plan = plan.with_effect(PendingEffect::named(Effect::ScheduleAnnouncement));
        }
    }

    plan
}

fn plan_drain_announcement(ctx: &ManagerContext, now_ms: u64) -> TransitionPlan<Machine> {
    const MIN_GAP_MS: u64 = 500;

    let due = ctx
        .last_announcement_at
        .is_none_or(|last| now_ms.saturating_sub(last) >= MIN_GAP_MS);

    if !due || ctx.announcement_queue.is_empty() {
        return TransitionPlan::new();
    }

    // Assertive entries always drain first within the same heartbeat.
    let head_idx = ctx
        .announcement_queue
        .iter()
        .position(|(_, priority)| matches!(priority, AnnouncePriority::Assertive))
        .unwrap_or(0);

    let priority = ctx.announcement_queue[head_idx].1;

    let intent = match priority {
        AnnouncePriority::Assertive => Effect::AnnounceAssertive,
        AnnouncePriority::Polite => Effect::AnnouncePolite,
    };

    let still_more = ctx.announcement_queue.len() > 1;

    let mut plan = TransitionPlan::context_only(move |ctx: &mut ManagerContext| {
        if head_idx < ctx.announcement_queue.len() {
            ctx.announcement_queue.remove(head_idx);
        }

        ctx.last_announcement_at = Some(now_ms);
    })
    .with_effect(PendingEffect::named(intent));

    if still_more {
        plan = plan.with_effect(PendingEffect::named(Effect::ScheduleAnnouncement));
    }

    plan
}

fn find_visible_dedup_match(ctx: &ManagerContext, config: &Config) -> Option<String> {
    let dedup = config.deduplicate || ctx.deduplicate_all;
    if !dedup {
        return None;
    }

    ctx.toasts
        .iter()
        .find(|entry| {
            entry.stage == EntryStage::Visible
                && entry.config.kind == config.kind
                && entry.config.title == config.title
                && entry.config.description == config.description
        })
        .map(|entry| entry.id.clone())
}

fn find_queued_dedup_match_index(ctx: &ManagerContext, config: &Config) -> Option<usize> {
    let dedup = config.deduplicate || ctx.deduplicate_all;
    if !dedup {
        return None;
    }

    ctx.queued.iter().position(|queued| {
        queued.kind == config.kind
            && queued.title == config.title
            && queued.description == config.description
    })
}

fn resolve_or_generate_id(ctx: &ManagerContext, config: &Config) -> String {
    if let Some(id) = config.id.as_ref() {
        return id.clone();
    }

    // Format auto-generated ids as `toast-<n>` using `next_id + 1` so the
    // first id is `toast-1` when `next_id` starts at 0.
    let mut s = String::with_capacity(8);

    s.push_str("toast-");

    push_decimal(&mut s, ctx.next_id.saturating_add(1));

    s
}

fn push_decimal(buf: &mut String, mut n: u64) {
    if n == 0 {
        buf.push('0');

        return;
    }

    // Buffer the digits in reverse, then push them in forward order.
    // u64::MAX is 20 digits; 24 is comfortably oversized.
    let mut digits = [0_u8; 24];

    let mut len = 0;

    while n > 0 {
        digits[len] = b'0' + (n % 10) as u8;

        len += 1;

        n /= 10;
    }

    for i in (0..len).rev() {
        buf.push(digits[i] as char);
    }
}

// ────────────────────────────────────────────────────────────────────
// Toaster — config-builder factory functions (agnostic core)
// ────────────────────────────────────────────────────────────────────

/// Zero-sized handle for building [`Config`] values without an active
/// manager [`Api`].
///
/// Adapters wrap this in their own `ToasterHandle` (Leptos / Dioxus) that
/// also dispatches the resulting `Config` through `Event::Add`. The
/// agnostic core deliberately stops at the **Config-construction** boundary
/// so it never has to know about an event-send closure.
#[derive(Clone, Copy, Debug, Default)]
pub struct Toaster;

impl Toaster {
    /// Builds an `Info` config.
    #[must_use]
    pub fn info(title: impl Into<String>, description: impl Into<String>) -> Config {
        Config::new(Kind::Info, title).description(description)
    }

    /// Builds a `Success` config.
    #[must_use]
    pub fn success(title: impl Into<String>, description: impl Into<String>) -> Config {
        Config::new(Kind::Success, title).description(description)
    }

    /// Builds a `Warning` config.
    #[must_use]
    pub fn warning(title: impl Into<String>, description: impl Into<String>) -> Config {
        Config::new(Kind::Warning, title).description(description)
    }

    /// Builds an `Error` config.
    #[must_use]
    pub fn error(title: impl Into<String>, description: impl Into<String>) -> Config {
        Config::new(Kind::Error, title).description(description)
    }

    /// Builds a persistent `Loading` config (`duration: None`).
    #[must_use]
    pub fn loading(title: impl Into<String>, description: impl Into<String>) -> Config {
        Config::new(Kind::Loading, title)
            .description(description)
            .duration(None)
    }
}

// ────────────────────────────────────────────────────────────────────
// Promise toast — agnostic-core data types
// ────────────────────────────────────────────────────────────────────

/// Content for a toast message — used by [`Promise`] and adapters that
/// transform success/error values into toast bodies on resolution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToastContent {
    /// Optional title.
    pub title: Option<String>,

    /// Optional description.
    pub description: Option<String>,
}

impl ToastContent {
    /// Builds content with the supplied title.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            description: None,
        }
    }

    /// Sets [`description`](Self::description).
    #[must_use]
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }
}

/// Configuration for a promise toast: a loading body shown immediately,
/// plus mappers that turn the future's `Ok(T)` / `Err(E)` into the final
/// success / error body.
///
/// The agnostic core only owns this data shape — actually spawning the
/// future, observing its result, and calling `Update` is adapter work
/// (`spawn_local` / `spawn`). See `spec/components/overlay/toast.md` §8.4.
pub struct Promise<T, E> {
    /// Body shown while the future is pending. Adapters dispatch this as a
    /// `Kind::Loading` toast.
    pub loading: ToastContent,

    /// Mapper invoked when the future resolves with `Ok(T)`. Returns the
    /// success-toast body.
    pub success: Callback<dyn Fn(T) -> ToastContent + Send + Sync>,

    /// Mapper invoked when the future resolves with `Err(E)`. Returns the
    /// error-toast body.
    pub error: Callback<dyn Fn(E) -> ToastContent + Send + Sync>,
}

impl<T, E> Clone for Promise<T, E> {
    fn clone(&self) -> Self {
        Self {
            loading: self.loading.clone(),
            success: self.success.clone(),
            error: self.error.clone(),
        }
    }
}

impl<T, E> Debug for Promise<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Promise")
            .field("loading", &self.loading)
            .field("success", &"<callback>")
            .field("error", &"<callback>")
            .finish()
    }
}

impl<T: 'static, E: 'static> Promise<T, E> {
    /// Builds a promise toast configuration with the supplied loading
    /// content and success/error mappers.
    pub fn new<S, F>(loading: ToastContent, success: S, error: F) -> Self
    where
        S: Fn(T) -> ToastContent + Send + Sync + 'static,
        F: Fn(E) -> ToastContent + Send + Sync + 'static,
    {
        Self {
            loading,
            success: Callback::new(success),
            error: Callback::new(error),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connected API surface for the toast manager.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a ManagerContext,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Returns `true` when every visible toast's auto-dismiss timer is
    /// globally paused.
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        matches!(self.state, State::Paused)
    }

    /// Returns the configured placement.
    #[must_use]
    pub const fn placement(&self) -> Placement {
        self.ctx.placement
    }

    /// Returns the swipe axis derived from the placement.
    #[must_use]
    pub const fn swipe_axis(&self) -> SwipeAxis {
        self.ctx.placement.swipe_axis()
    }

    /// Returns the ids of toasts that count against `max_visible` (i.e.
    /// `EntryStage::Visible`), in stacking order.
    #[must_use]
    pub fn visible_ids(&self) -> Vec<&str> {
        self.ctx
            .toasts
            .iter()
            .filter(|entry| entry.stage == EntryStage::Visible)
            .map(|entry| entry.id.as_str())
            .collect()
    }

    /// Returns the number of toasts currently waiting in the admission
    /// queue (over `max_visible`).
    #[must_use]
    pub fn queued_len(&self) -> usize {
        self.ctx.queued.len()
    }

    /// Returns the number of pending announcement entries.
    #[must_use]
    pub fn announcement_backlog(&self) -> usize {
        self.ctx.announcement_queue.len()
    }

    /// Returns attributes for the manager's outer container element.
    ///
    /// Stamped data attributes:
    /// * `data-ars-scope="toast-provider"` and `data-ars-part="root"`
    /// * `data-ars-placement` — the configured placement token
    /// * `data-ars-paused="true"` when global pause is active
    /// * `data-ars-overlap` (presence-only) when overlap mode is enabled
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.props.id.clone())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-placement"), self.ctx.placement.as_str())
            .set(
                HtmlAttr::Data("ars-paused"),
                if self.is_paused() { "true" } else { "false" },
            );

        if self.ctx.overlap {
            attrs.set_bool(HtmlAttr::Data("ars-overlap"), true);
        }

        attrs
    }

    /// Returns attributes for the polite or assertive `aria-live` region
    /// shell rendered by the surrounding adapter.
    ///
    /// Both regions share `data-ars-scope="toast"` and
    /// `data-ars-part="region"` so CSS selectors and test queries for the
    /// toast surface match the regions and the per-toast roots uniformly.
    #[must_use]
    pub fn region_attrs(&self, assertive: bool) -> AttrMap {
        region_attrs(
            &self.ctx.messages,
            &self.ctx.locale,
            if assertive {
                RegionPart::Assertive
            } else {
                RegionPart::Polite
            },
        )
    }

    /// Dispatches an `Add` event with the supplied config.
    pub fn add(&self, config: Config) {
        (self.send)(Event::Add(config));
    }

    /// Dispatches an `Update` event for the supplied id.
    pub fn update(&self, id: impl Into<String>, config: Config) {
        (self.send)(Event::Update(id.into(), config));
    }

    /// Dispatches a `Remove` event for the supplied id.
    pub fn dismiss(&self, id: impl Into<String>) {
        (self.send)(Event::Remove(id.into()));
    }

    /// Dispatches a `DismissAll` event.
    pub fn dismiss_all(&self) {
        (self.send)(Event::DismissAll);
    }

    /// Dispatches a `PauseAll` event.
    pub fn pause_all(&self) {
        (self.send)(Event::PauseAll);
    }

    /// Dispatches a `ResumeAll` event.
    pub fn resume_all(&self) {
        (self.send)(Event::ResumeAll);
    }

    /// Dispatches a `DrainAnnouncement` event with the adapter's current
    /// clock timestamp (ms).
    pub fn drain_announcement(&self, now_ms: u64) {
        (self.send)(Event::DrainAnnouncement { now_ms });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Region helper (canonical scope = "toast")
// ────────────────────────────────────────────────────────────────────

/// Returns attributes for the SSR-rendered `aria-live` region container.
///
/// This is the **single canonical** region helper — both the manager's
/// connect-time [`Api::region_attrs`] and adapter-side direct callers go
/// through it so the rendered scope is always `data-ars-scope="toast"`.
/// Per `spec/components/overlay/toast.md` §4, every toast provider renders
/// **two** regions in the server HTML — one polite (`Info`/`Success`/
/// `Loading`) and one assertive (`Warning`/`Error`).
#[must_use]
pub fn region_attrs(messages: &Messages, locale: &Locale, part: RegionPart) -> AttrMap {
    let mut attrs = AttrMap::new();

    let assertive = part.is_assertive();

    let label = (messages.region_label)(locale);

    attrs
        .set(HtmlAttr::Role, if assertive { "alert" } else { "status" })
        .set(
            HtmlAttr::Aria(AriaAttr::Live),
            if assertive { "assertive" } else { "polite" },
        )
        .set(HtmlAttr::Aria(AriaAttr::Atomic), "false")
        .set(HtmlAttr::Aria(AriaAttr::Label), label)
        .set(HtmlAttr::Data("ars-scope"), "toast")
        .set(HtmlAttr::Data("ars-part"), "region")
        .set(
            HtmlAttr::Data("ars-live"),
            if assertive { "assertive" } else { "polite" },
        );

    attrs
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, vec::Vec};
    use core::cell::RefCell;

    use ars_core::Service;
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "toaster".to_string(),
            ..Props::default()
        }
    }

    fn fresh_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn add_config(kind: Kind, title: &str) -> Config {
        Config::new(kind, title).description("body")
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn effect_names(result: &ars_core::SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    // ── Props builder ───────────────────────────────────────────────

    #[test]
    fn props_new_returns_defaults() {
        let p = Props::new();

        assert_eq!(p.placement, Placement::BottomEnd);
        assert_eq!(p.max_visible, 5);
        assert_eq!(p.gap, 16.0);
        assert_eq!(p.remove_delay, Duration::from_millis(200));
        assert!(p.pause_on_hover);
        assert!(p.pause_on_page_idle);
        assert!(!p.deduplicate_all);
        assert!(!p.overlap);
        assert!(p.hotkey.is_none());
    }

    #[test]
    fn props_max_visible_clamps_zero_to_one() {
        assert_eq!(Props::new().max_visible(0).max_visible, 1);
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let custom_durations = DefaultDurations {
            info: Duration::from_millis(1_000),
            success: Duration::from_millis(2_000),
            warning: Duration::from_millis(3_000),
            error: Duration::from_millis(4_000),
            loading: Some(Duration::from_millis(5_000)),
        };

        let p = Props::new()
            .id("toaster-builder")
            .placement(Placement::TopCenter)
            .max_visible(3)
            .gap(8.0)
            .remove_delay(Duration::from_millis(100))
            .default_durations(custom_durations)
            .deduplicate_all(true)
            .pause_on_hover(false)
            .pause_on_page_idle(false)
            .offsets(EdgeOffsets {
                top: 4.0,
                bottom: 4.0,
                ..EdgeOffsets::default()
            })
            .overlap(true)
            .hotkey(Hotkey::char('t').with_alt());

        assert_eq!(p.id, "toaster-builder");
        assert_eq!(p.placement, Placement::TopCenter);
        assert_eq!(p.max_visible, 3);
        assert_eq!(p.gap, 8.0);
        assert_eq!(p.remove_delay, Duration::from_millis(100));
        assert_eq!(p.default_durations, custom_durations);
        assert!(p.deduplicate_all);
        assert!(!p.pause_on_hover);
        assert!(!p.pause_on_page_idle);
        assert_eq!(p.offsets.top, 4.0);
        assert!(p.overlap);
        assert_eq!(p.hotkey, Some(Hotkey::char('t').with_alt()));
    }

    #[test]
    fn config_builder_round_trips_every_field() {
        let cfg = Config::new(Kind::Error, "boom")
            .description("stack")
            .id("explicit-id")
            .duration(Some(Duration::from_millis(1_500)))
            .dismissible(false)
            .deduplicate(true);

        assert_eq!(cfg.kind, Kind::Error);
        assert_eq!(cfg.title.as_deref(), Some("boom"));
        assert_eq!(cfg.description.as_deref(), Some("stack"));
        assert_eq!(cfg.id.as_deref(), Some("explicit-id"));
        assert_eq!(cfg.duration, Some(Duration::from_millis(1_500)));
        assert!(!cfg.dismissible);
        assert!(cfg.deduplicate);
    }

    #[test]
    fn config_partial_eq_compares_every_field() {
        // Identical configs are equal.
        let a = Config::new(Kind::Info, "hi").description("body");
        let b = Config::new(Kind::Info, "hi").description("body");

        assert_eq!(a, b);

        // Differing kind / title / description / id / duration /
        // dismissible / deduplicate each break equality.
        assert_ne!(
            a,
            Config::new(Kind::Success, "hi").description("body"),
            "kind difference must break eq"
        );
        assert_ne!(
            a,
            Config::new(Kind::Info, "different").description("body"),
            "title difference must break eq"
        );
        assert_ne!(
            a,
            Config::new(Kind::Info, "hi").description("different"),
            "description difference must break eq"
        );
        assert_ne!(
            a,
            Config::new(Kind::Info, "hi").description("body").id("x"),
            "id difference must break eq"
        );
        assert_ne!(
            a,
            Config::new(Kind::Info, "hi")
                .description("body")
                .duration(Some(Duration::from_secs(1))),
            "duration difference must break eq"
        );
        assert_ne!(
            a,
            Config::new(Kind::Info, "hi")
                .description("body")
                .dismissible(false),
            "dismissible difference must break eq"
        );
        assert_ne!(
            a,
            Config::new(Kind::Info, "hi")
                .description("body")
                .deduplicate(true),
            "deduplicate difference must break eq"
        );

        // on_pause_change Callback equality is pointer identity.
        let cb = ars_core::callback(|_paused: bool| {});

        let with_cb_a = Config {
            on_pause_change: Some(cb.clone()),
            ..Config::new(Kind::Info, "hi").description("body")
        };

        let with_cb_b = Config {
            on_pause_change: Some(cb),
            ..Config::new(Kind::Info, "hi").description("body")
        };

        assert_eq!(
            with_cb_a, with_cb_b,
            "shared Arc-pointer callbacks compare equal"
        );

        let with_diff_cb = Config {
            on_pause_change: Some(ars_core::callback(|_paused: bool| {})),
            ..Config::new(Kind::Info, "hi").description("body")
        };

        assert_ne!(
            with_cb_a, with_diff_cb,
            "different Arc-pointer callbacks compare unequal"
        );
        assert_ne!(
            a, with_cb_a,
            "Some(cb) vs None for on_pause_change must break eq"
        );
    }

    #[test]
    fn push_decimal_handles_zero_and_multi_digit_values() {
        let mut buf = String::new();

        push_decimal(&mut buf, 0);

        assert_eq!(buf, "0");

        let mut buf = String::new();

        push_decimal(&mut buf, 7);

        assert_eq!(buf, "7");

        let mut buf = String::new();

        push_decimal(&mut buf, 1234);

        assert_eq!(buf, "1234");
    }

    #[test]
    fn placement_swipe_axis_derives_from_position() {
        for placement in [Placement::TopCenter, Placement::BottomCenter] {
            assert_eq!(placement.swipe_axis(), SwipeAxis::Vertical);
        }

        for placement in [
            Placement::TopStart,
            Placement::TopEnd,
            Placement::BottomStart,
            Placement::BottomEnd,
            Placement::TopLeft,
            Placement::TopRight,
            Placement::BottomLeft,
            Placement::BottomRight,
        ] {
            assert_eq!(placement.swipe_axis(), SwipeAxis::Horizontal);
        }
    }

    #[test]
    fn placement_as_str_round_trip() {
        for (placement, expected) in [
            (Placement::TopStart, "top-start"),
            (Placement::TopCenter, "top-center"),
            (Placement::TopEnd, "top-end"),
            (Placement::BottomStart, "bottom-start"),
            (Placement::BottomCenter, "bottom-center"),
            (Placement::BottomEnd, "bottom-end"),
            (Placement::TopLeft, "top-left"),
            (Placement::TopRight, "top-right"),
            (Placement::BottomLeft, "bottom-left"),
            (Placement::BottomRight, "bottom-right"),
        ] {
            assert_eq!(placement.as_str(), expected);
        }
    }

    #[test]
    fn kind_announce_priority_routes_assertive_kinds_to_assertive_region() {
        for kind in [Kind::Info, Kind::Success, Kind::Loading] {
            assert_eq!(kind.announce_priority(), AnnouncePriority::Polite);
        }

        for kind in [Kind::Warning, Kind::Error] {
            assert_eq!(kind.announce_priority(), AnnouncePriority::Assertive);
        }
    }

    #[test]
    fn default_durations_round_trip() {
        let d = DefaultDurations::default();

        assert_eq!(d.for_kind(Kind::Info), Some(Duration::from_secs(5)));
        assert_eq!(d.for_kind(Kind::Success), Some(Duration::from_secs(5)));
        assert_eq!(d.for_kind(Kind::Warning), Some(Duration::from_secs(5)));
        assert_eq!(d.for_kind(Kind::Error), Some(Duration::from_secs(8)));
        assert_eq!(d.for_kind(Kind::Loading), None);
    }

    // ── Add / Remove / Update ───────────────────────────────────────

    #[test]
    fn add_admits_when_under_max_visible() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Add(add_config(Kind::Info, "hello")));

        assert_eq!(service.context().toasts.len(), 1);
        assert_eq!(service.context().toasts[0].config.kind, Kind::Info);
        assert!(service.context().queued.is_empty());
        // `Add` enqueues the announcement and schedules the heartbeat,
        // but does NOT emit `AnnouncePolite` directly — see the
        // `add_does_not_double_announce` regression test for the
        // motivating bug. The announce effect fires from
        // `DrainAnnouncement` once the adapter's heartbeat ticks.
        assert_eq!(effect_names(&result), vec![Effect::ScheduleAnnouncement]);
        assert_eq!(service.context().announcement_queue.len(), 1);
        assert_eq!(
            service.context().announcement_queue[0].1,
            AnnouncePriority::Polite
        );
    }

    /// Regression test for the P1 review finding: every admitted toast
    /// previously emitted `AnnouncePolite`/`AnnounceAssertive` *and*
    /// enqueued itself, so the subsequent `DrainAnnouncement` produced
    /// a second announce effect for the same toast (double-announce in
    /// the live region).
    #[test]
    fn add_does_not_double_announce() {
        let mut service = fresh_service(test_props());

        let add_result = service.send(Event::Add(add_config(Kind::Info, "hello")));

        // Admission emits no announce effect — only the heartbeat
        // schedule.
        assert!(
            !effect_names(&add_result).contains(&Effect::AnnouncePolite),
            "admission must not emit AnnouncePolite directly"
        );
        assert!(
            !effect_names(&add_result).contains(&Effect::AnnounceAssertive),
            "admission must not emit AnnounceAssertive directly"
        );
        assert!(effect_names(&add_result).contains(&Effect::ScheduleAnnouncement));
        assert_eq!(service.context().announcement_queue.len(), 1);

        // The adapter heartbeat fires DrainAnnouncement → ONE announce.
        let drain_result = service.send(Event::DrainAnnouncement { now_ms: 0 });
        let drain_effects = effect_names(&drain_result);

        assert_eq!(
            drain_effects
                .iter()
                .filter(|e| matches!(e, Effect::AnnouncePolite | Effect::AnnounceAssertive))
                .count(),
            1,
            "drain emits exactly one announce for the queued toast"
        );
        assert!(service.context().announcement_queue.is_empty());

        // Subsequent drains (within or beyond the gap) emit nothing —
        // the toast is announced exactly once total.
        let stale = service.send(Event::DrainAnnouncement { now_ms: 1_000 });

        assert!(stale.pending_effects.is_empty());
    }

    #[test]
    fn add_assigns_auto_generated_id_when_config_has_none() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "hello"))));

        let entry_id = &service.context().toasts[0].id;

        assert!(entry_id.starts_with("toast-"));
    }

    #[test]
    fn add_overflow_pushes_to_queue() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "first"))));

        let result = service.send(Event::Add(add_config(Kind::Info, "second")));

        assert_eq!(service.context().toasts.len(), 1);
        assert_eq!(service.context().queued.len(), 1);

        // Overflow must NOT enqueue an announcement until the toast actually
        // becomes visible.
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn remove_then_advance_promotes_queued_toast() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "first"))));
        drop(service.send(Event::Add(add_config(Kind::Error, "second"))));

        let visible_id = service.context().toasts[0].id.clone();

        // First Remove only marks the live entry Dismissing; the queued
        // toast is still in the queue at this point.
        drop(service.send(Event::Remove(visible_id.clone())));

        assert_eq!(service.context().toasts.len(), 1);
        assert_eq!(service.context().queued.len(), 1);

        let result = service.send(Event::HideQueueAdvance(visible_id));

        assert_eq!(service.context().toasts.len(), 1);
        assert_eq!(service.context().queued.len(), 0);
        assert_eq!(service.context().toasts[0].config.kind, Kind::Error);
        // Promotion enqueues the announcement (assertive) but does NOT
        // emit `AnnounceAssertive` directly — the announce effect fires
        // from `DrainAnnouncement`. See `add_does_not_double_announce`
        // for the motivating regression.
        assert!(!effect_names(&result).contains(&Effect::AnnounceAssertive));

        let announcement_queue = &service.context().announcement_queue;

        assert!(
            announcement_queue
                .iter()
                .any(|(_, priority)| *priority == AnnouncePriority::Assertive),
            "the promoted error toast must be enqueued for an assertive announcement"
        );
    }

    #[test]
    fn remove_unknown_id_is_noop() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "live"))));

        let toasts_before = service.context().toasts.clone();
        let queued_before = service.context().queued.clone();

        let result = service.send(Event::Remove("does-not-exist".to_string()));

        assert_eq!(service.context().toasts, toasts_before);
        assert_eq!(service.context().queued, queued_before);
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn remove_drops_a_queued_toast_outright() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "live"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "queued").id("custom-q"))));

        assert_eq!(service.context().queued.len(), 1);

        drop(service.send(Event::Remove("custom-q".to_string())));

        assert!(service.context().queued.is_empty());

        // The live entry is untouched.
        assert_eq!(service.context().toasts.len(), 1);
        assert_eq!(service.context().toasts[0].stage, EntryStage::Visible);
    }

    #[test]
    fn update_replaces_existing_entry_and_announces() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "loading"))));

        let id = service.context().toasts[0].id.clone();

        let result = service.send(Event::Update(
            id.clone(),
            Config::new(Kind::Success, "done").description("ok"),
        ));

        let entry = service
            .context()
            .toasts
            .iter()
            .find(|e| e.id == id)
            .unwrap();

        assert_eq!(entry.config.kind, Kind::Success);
        assert_eq!(entry.config.title.as_deref(), Some("done"));

        // `Update` enqueues the announcement; the announce effect itself
        // fires from `DrainAnnouncement`, never directly from `Update`.
        assert!(!effect_names(&result).contains(&Effect::AnnouncePolite));

        // Two announcements queued: the original `Add` and this `Update`.
        assert_eq!(service.context().announcement_queue.len(), 2);
        assert!(
            service
                .context()
                .announcement_queue
                .iter()
                .all(|(_, priority)| *priority == AnnouncePriority::Polite),
            "both announcements should be polite (Info, then Success)"
        );
    }

    /// Regression test for the P1 review finding: a second `Add` that
    /// supplies the same explicit id as a live entry must NOT push a
    /// duplicate row. Two entries sharing an id silently break
    /// `Update(id)` / `Remove(id)` first-match lookups (they would only
    /// touch one of the two), leaving the other on screen indefinitely.
    /// The fix routes the second `Add` through `plan_update`.
    #[test]
    fn add_with_duplicate_explicit_id_routes_to_update_for_live_entry() {
        let mut service = fresh_service(test_props());

        drop(
            service.send(Event::Add(
                Config::new(Kind::Info, "first")
                    .id("custom-id")
                    .description("body"),
            )),
        );
        let initial_len = service.context().toasts.len();

        assert_eq!(initial_len, 1);
        assert_eq!(service.context().toasts[0].id, "custom-id");

        // Same explicit id, different content. Should update, not stack.
        let result = service.send(Event::Add(
            Config::new(Kind::Success, "updated")
                .id("custom-id")
                .description("new body"),
        ));

        assert_eq!(
            service.context().toasts.len(),
            initial_len,
            "duplicate explicit id must not push a second row"
        );

        let entry = &service.context().toasts[0];

        assert_eq!(entry.id, "custom-id");
        assert_eq!(entry.config.kind, Kind::Success);
        assert_eq!(entry.config.title.as_deref(), Some("updated"));
        assert_eq!(entry.config.description.as_deref(), Some("new body"));

        // Update path emits no immediate announce (queue-only flow).
        assert!(
            !effect_names(&result).contains(&Effect::AnnouncePolite)
                && !effect_names(&result).contains(&Effect::AnnounceAssertive)
        );
    }

    /// Sibling P1 regression: same fix on the queued slot path.
    #[test]
    fn add_with_duplicate_explicit_id_replaces_queued_slot() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "live"))));
        drop(
            service.send(Event::Add(
                Config::new(Kind::Info, "queued-original")
                    .id("queued-id")
                    .description("body"),
            )),
        );

        assert_eq!(service.context().queued.len(), 1);
        assert_eq!(service.context().queued[0].id.as_deref(), Some("queued-id"));

        // Same explicit id, different kind. Replaces the queued slot in
        // place, preserving the id.
        drop(
            service.send(Event::Add(
                Config::new(Kind::Success, "queued-updated")
                    .id("queued-id")
                    .description("body"),
            )),
        );

        assert_eq!(
            service.context().queued.len(),
            1,
            "duplicate explicit id must not append a queued duplicate"
        );
        assert_eq!(service.context().queued[0].kind, Kind::Success);
        assert_eq!(
            service.context().queued[0].title.as_deref(),
            Some("queued-updated")
        );
        assert_eq!(service.context().queued[0].id.as_deref(), Some("queued-id"));
    }

    /// P1 corner case: explicit-id match takes priority over content-
    /// based dedup. The id is the addressing contract; dedup-by-content
    /// is a secondary optimization. Without the fix, a third `Add`
    /// whose content matched an existing entry would route through
    /// content-dedup → `plan_update("primary", ...)` and silently
    /// overwrite the wrong toast. With the fix, the explicit-id match
    /// fires first and updates `secondary` instead.
    #[test]
    fn add_with_duplicate_explicit_id_wins_over_content_dedup() {
        let mut service = fresh_service(test_props());

        // Two distinct entries with distinct content.
        drop(
            service.send(Event::Add(
                Config::new(Kind::Info, "primary-content")
                    .id("primary")
                    .description("body"),
            )),
        );
        drop(
            service.send(Event::Add(
                Config::new(Kind::Info, "secondary-content")
                    .id("secondary")
                    .description("body"),
            )),
        );

        assert_eq!(service.context().toasts.len(), 2);

        // Now `Add` again with `id="secondary"` carrying content that
        // matches *primary* and `deduplicate=true`. Content-dedup would
        // route to `primary`; the explicit-id match must win and route
        // to `secondary` instead.
        drop(
            service.send(Event::Add(
                Config::new(Kind::Success, "primary-content")
                    .id("secondary")
                    .description("body")
                    .deduplicate(true),
            )),
        );

        assert_eq!(service.context().toasts.len(), 2, "no new row was added");

        let secondary = service
            .context()
            .toasts
            .iter()
            .find(|e| e.id == "secondary")
            .expect("secondary id still present");
        assert_eq!(
            secondary.config.kind,
            Kind::Success,
            "secondary was updated (explicit-id match)"
        );
        assert_eq!(secondary.config.title.as_deref(), Some("primary-content"));

        let primary = service
            .context()
            .toasts
            .iter()
            .find(|e| e.id == "primary")
            .expect("primary id still present");
        assert_eq!(
            primary.config.kind,
            Kind::Info,
            "primary was untouched (content-dedup did NOT win)"
        );
        assert_eq!(primary.config.title.as_deref(), Some("primary-content"));
    }

    /// Regression test for the P2 review finding: `Update(id)` arriving
    /// in the window between `Remove(id)` and `HideQueueAdvance(id)`
    /// previously forced the entry's stage back to `Visible`, briefly
    /// reviving the toast before the pending advance removed it again.
    /// The fix preserves the existing stage so an in-flight dismiss
    /// stays consistent.
    #[test]
    fn update_does_not_revive_dismissing_entry() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "loading"))));

        let id = service.context().toasts[0].id.clone();

        // Mark Dismissing.
        drop(service.send(Event::Remove(id.clone())));

        assert_eq!(service.context().toasts[0].stage, EntryStage::Dismissing);

        // Update arrives during the dismiss-animation window.
        drop(service.send(Event::Update(
            id.clone(),
            Config::new(Kind::Success, "done").description("ok"),
        )));

        let entry = service
            .context()
            .toasts
            .iter()
            .find(|e| e.id == id)
            .expect("entry still tracked");

        assert_eq!(
            entry.stage,
            EntryStage::Dismissing,
            "Update must NOT revive a Dismissing entry — the lifecycle is owned by the dismiss flow"
        );

        // The config IS replaced — adapters that want to reflect updated
        // content during the exit animation can; the lifecycle just
        // doesn't bounce back.
        assert_eq!(entry.config.kind, Kind::Success);
        assert_eq!(entry.config.title.as_deref(), Some("done"));
    }

    /// Sibling P2 fix: an `Update` on a Dismissing entry must NOT
    /// announce. The toast is animating out — re-announcing the new
    /// content would either talk over the screen reader's existing
    /// announcement or be cut short when the toast disappears.
    #[test]
    fn update_on_dismissing_entry_does_not_announce() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "x"))));

        let id = service.context().toasts[0].id.clone();

        // Mark Dismissing — `plan_remove` also drops the pending
        // announcement for this id.
        drop(service.send(Event::Remove(id.clone())));

        assert!(service.context().announcement_queue.is_empty());

        let result = service.send(Event::Update(
            id,
            Config::new(Kind::Success, "y").description("z"),
        ));

        assert!(
            service.context().announcement_queue.is_empty(),
            "Update on a Dismissing entry must not enqueue an announcement"
        );
        assert!(result.pending_effects.is_empty());
    }

    /// Regression test for the P1.A review finding: `plan_update` used
    /// to write the incoming config directly without applying the
    /// `DefaultDurations` fallback. An `Update(id, Config::new(...))` —
    /// or any dedup-routed `Add` whose builder left `duration: None` —
    /// silently flipped an auto-dismissing toast into a persistent one.
    #[test]
    fn update_with_unset_duration_falls_back_to_default() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "loading"))));

        let id = service.context().toasts[0].id.clone();
        let original_duration = service.context().toasts[0].config.duration;

        assert_eq!(original_duration, Some(Duration::from_secs(5)));

        // `Config::new(...)` leaves `duration: None`.
        drop(service.send(Event::Update(
            id.clone(),
            Config::new(Kind::Success, "done").description("ok"),
        )));

        let entry = service
            .context()
            .toasts
            .iter()
            .find(|e| e.id == id)
            .expect("entry still tracked");

        // Defaults were applied — toast still auto-dismisses.
        assert_eq!(
            entry.config.duration,
            Some(Duration::from_secs(5)),
            "Update with `duration: None` must fall back to \
             `default_durations.for_kind(kind)` rather than silently \
             turn the toast persistent"
        );
    }

    /// P1.A regression sibling: an explicit `duration` on Update is NOT
    /// overridden by the fallback. Only `None` is filled.
    #[test]
    fn update_preserves_explicit_duration() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "x"))));

        let id = service.context().toasts[0].id.clone();

        let explicit = Some(Duration::from_secs(12));

        drop(
            service.send(Event::Update(
                id.clone(),
                Config::new(Kind::Info, "x")
                    .description("body")
                    .duration(explicit),
            )),
        );

        let entry = service
            .context()
            .toasts
            .iter()
            .find(|e| e.id == id)
            .expect("entry still tracked");

        assert_eq!(
            entry.config.duration, explicit,
            "explicit duration on Update must NOT be overridden by the fallback"
        );
    }

    /// P1.A regression: a dedup-routed `Add` (which internally goes
    /// through `plan_update`) must not lose the default duration. Before
    /// the fix, the second `Add` with `Config::new` would dedup to the
    /// existing entry but leave `duration: None`.
    #[test]
    fn dedup_routed_add_keeps_default_duration() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "same").deduplicate(true))));

        // Second Add with same content → routes through `plan_update`
        // via content-dedup. `add_config` returns a `Config::new`, so
        // `duration` starts as `None`.
        drop(service.send(Event::Add(add_config(Kind::Info, "same").deduplicate(true))));

        assert_eq!(service.context().toasts.len(), 1);
        assert_eq!(
            service.context().toasts[0].config.duration,
            Some(Duration::from_secs(5)),
            "dedup-routed Update path must still fill the default duration"
        );
    }

    #[test]
    fn dedup_visible_resets_existing_instead_of_stacking() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "same").deduplicate(true))));

        let initial_id = service.context().toasts[0].id.clone();
        let initial_len = service.context().toasts.len();

        drop(service.send(Event::Add(add_config(Kind::Info, "same").deduplicate(true))));

        // Same length — no duplicate entry.
        assert_eq!(service.context().toasts.len(), initial_len);
        assert_eq!(service.context().toasts[0].id, initial_id);

        // Re-announces because Update fires on the matched entry.
        assert!(service.context().announcement_queue.len() >= 2);
    }

    #[test]
    fn dedup_queued_replaces_in_place() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            deduplicate_all: true,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "live"))));
        drop(service.send(Event::Add(add_config(Kind::Error, "queued"))));

        assert_eq!(service.context().queued.len(), 1);

        let queued_id_before = service.context().queued[0].id.clone();

        // Same kind+title+description on a queued entry — must be replaced
        // in place, not stacked behind it.
        drop(service.send(Event::Add(add_config(Kind::Error, "queued"))));

        assert_eq!(service.context().queued.len(), 1);

        // Queued slot's id is preserved.
        assert_eq!(service.context().queued[0].id, queued_id_before);
    }

    #[test]
    fn dedup_all_flag_overrides_per_config_dedup_off() {
        let mut service = fresh_service(Props {
            deduplicate_all: true,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "same"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "same"))));

        assert_eq!(service.context().toasts.len(), 1);
    }

    #[test]
    fn add_falls_back_to_default_durations_for_kind() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(Config::new(Kind::Error, "boom"))));

        assert_eq!(
            service.context().toasts[0].config.duration,
            Some(Duration::from_secs(8))
        );
    }

    #[test]
    fn add_with_explicit_duration_does_not_overwrite() {
        let mut service = fresh_service(test_props());

        drop(
            service.send(Event::Add(
                Config::new(Kind::Info, "hi")
                    .description("body")
                    .duration(Some(Duration::from_secs(2))),
            )),
        );

        // Explicit duration must NOT be overwritten by `default_durations`.
        assert_eq!(
            service.context().toasts[0].config.duration,
            Some(Duration::from_secs(2))
        );
    }

    #[test]
    fn update_on_queued_entry_replaces_in_place_without_announce() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        // Live + queued.
        drop(service.send(Event::Add(add_config(Kind::Info, "live"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "queued").id("q1"))));

        // Drain announcements so we can see what Update emits.
        drop(service.send(Event::DrainAnnouncement { now_ms: 0 }));
        drop(service.send(Event::DrainAnnouncement { now_ms: 1_000 }));

        assert!(service.context().announcement_queue.is_empty());

        let result = service.send(Event::Update(
            "q1".to_string(),
            Config::new(Kind::Success, "queued-updated").description("body"),
        ));

        // The queued slot was updated in place — still queued, no toast
        // surfaced.
        assert_eq!(service.context().queued.len(), 1);
        assert_eq!(
            service.context().queued[0].title.as_deref(),
            Some("queued-updated")
        );
        assert_eq!(service.context().queued[0].kind, Kind::Success);

        // Update on a queued entry MUST NOT announce — the toast hasn't
        // surfaced yet, so the live region has nothing to read.
        assert!(result.pending_effects.is_empty());
        assert!(service.context().announcement_queue.is_empty());
    }

    // ── Pause / Resume / DismissAll / SetVisibility ────────────────

    #[test]
    fn pause_all_emits_pause_effect() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::PauseAll);

        assert_eq!(service.state(), &State::Paused);
        assert!(service.context().paused_all);
        assert_eq!(effect_names(&result), vec![Effect::PauseAllTimers]);
    }

    #[test]
    fn pause_all_when_already_paused_is_noop() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::PauseAll));

        let result = service.send(Event::PauseAll);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn resume_all_when_already_active_is_noop() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::ResumeAll);

        assert_eq!(service.state(), &State::Active);
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn hide_queue_advance_at_capacity_promotes_nothing() {
        // max_visible = 2, three toasts in flight: two visible, one queued.
        let mut service = fresh_service(Props {
            max_visible: 2,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "b"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "c"))));

        let id_a = service.context().toasts[0].id.clone();
        let id_b = service.context().toasts[1].id.clone();

        assert_eq!(service.context().queued.len(), 1);

        // Mark BOTH visible toasts dismissing first (Remove keeps them in
        // ctx.toasts as Dismissing) — capacity is still consumed.
        drop(service.send(Event::Remove(id_a.clone())));
        drop(service.send(Event::Remove(id_b.clone())));

        // Now advance only one. After removal, one Dismissing entry
        // remains, so visible_after_remove == 1 < max_visible = 2 and the
        // queued entry IS promoted. Verify the active path.
        drop(service.send(Event::HideQueueAdvance(id_a)));

        assert_eq!(service.context().toasts.len(), 2);
        assert_eq!(service.context().queued.len(), 0);

        // Now advance the second; queue is empty, nothing to promote, so
        // the `else { None }` branch in `plan_hide_queue_advance` fires.
        let before_count = service.context().toasts.len();

        let result = service.send(Event::HideQueueAdvance(id_b));

        // Removal happened, but no new toast was admitted from the queue
        // (it's empty), so no announcement effect.
        assert_eq!(service.context().toasts.len(), before_count - 1);
        assert!(
            result.pending_effects.is_empty(),
            "no queued toast to promote → no announce effect"
        );
    }

    #[test]
    fn resume_all_emits_resume_effect() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::PauseAll));

        let result = service.send(Event::ResumeAll);

        assert_eq!(service.state(), &State::Active);
        assert!(!service.context().paused_all);
        assert_eq!(effect_names(&result), vec![Effect::ResumeAllTimers]);
    }

    #[test]
    fn dismiss_all_marks_every_visible_dismissing_and_clears_queue() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "b"))));

        let result = service.send(Event::DismissAll);

        assert!(service.context().queued.is_empty());
        assert_eq!(service.context().toasts.len(), 1);
        assert_eq!(service.context().toasts[0].stage, EntryStage::Dismissing);
        assert_eq!(effect_names(&result), vec![Effect::DismissAllToasts]);
    }

    /// Regression test for the P2 review finding: `DismissAll` cleared
    /// the admission queue and marked visible toasts as dismissing, but
    /// it did NOT clear `announcement_queue`. Pending `DrainAnnouncement`
    /// ticks would then announce content the user just dismissed —
    /// stale screen-reader output for invisible toasts.
    #[test]
    fn dismiss_all_clears_announcement_queue() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Error, "b"))));
        drop(service.send(Event::Add(add_config(Kind::Success, "c"))));
        assert_eq!(service.context().announcement_queue.len(), 3);

        let result = service.send(Event::DismissAll);

        assert_eq!(effect_names(&result), vec![Effect::DismissAllToasts]);
        assert!(
            service.context().announcement_queue.is_empty(),
            "DismissAll must drop pending announcements so the screen reader \
             does not announce dismissed content"
        );

        // A subsequent drain emits no announce effect — there's nothing
        // left to announce.
        let drain = service.send(Event::DrainAnnouncement { now_ms: 0 });
        assert!(drain.pending_effects.is_empty());
    }

    /// Sibling fix for the P2 class — `Remove(id)` should drop any
    /// pending announcement for that specific id, otherwise a fast
    /// dismiss before the heartbeat fires would announce the toast
    /// after the user removed it.
    #[test]
    fn remove_drops_pending_announcement_for_that_id() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "b"))));
        let a_id = service.context().toasts[0].id.clone();
        let b_id = service.context().toasts[1].id.clone();
        assert_eq!(service.context().announcement_queue.len(), 2);

        drop(service.send(Event::Remove(a_id.clone())));

        // Only `b`'s announcement remains queued.
        let queue_ids: Vec<&str> = service
            .context()
            .announcement_queue
            .iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(queue_ids, vec![b_id.as_str()]);
        assert!(!queue_ids.contains(&a_id.as_str()));
    }

    /// `HideQueueAdvance` removes the toast from `ctx.toasts`; any
    /// pending announcement for that id should disappear with it.
    #[test]
    fn hide_queue_advance_drops_pending_announcement_for_removed_id() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        let a_id = service.context().toasts[0].id.clone();
        assert_eq!(service.context().announcement_queue.len(), 1);

        // Mark `a` Dismissing then advance — the toast leaves
        // `ctx.toasts` and its pending announcement should leave
        // `announcement_queue` together.
        drop(service.send(Event::Remove(a_id.clone())));
        // (Remove already drops the queued announcement for the id;
        // re-Adding a fresh one tests the HideQueueAdvance path
        // independently.)
        drop(service.send(Event::Add(add_config(Kind::Info, "a-again"))));
        let new_id = service.context().toasts[1].id.clone();
        // New toast is queued for announcement.
        assert!(
            service
                .context()
                .announcement_queue
                .iter()
                .any(|(id, _)| id == &new_id)
        );

        drop(service.send(Event::HideQueueAdvance(new_id.clone())));

        // The dismissing toast is gone from both lists.
        assert!(
            service.context().toasts.iter().all(|e| e.id != new_id),
            "HideQueueAdvance removes the entry from ctx.toasts"
        );
        assert!(
            service
                .context()
                .announcement_queue
                .iter()
                .all(|(id, _)| id != &new_id),
            "HideQueueAdvance must also drop the pending announcement"
        );
    }

    #[test]
    fn set_visibility_false_pauses() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::SetVisibility(false));

        assert_eq!(service.state(), &State::Paused);
        assert_eq!(effect_names(&result), vec![Effect::PauseAllTimers]);
    }

    #[test]
    fn set_visibility_true_after_pause_resumes() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetVisibility(false)));

        let result = service.send(Event::SetVisibility(true));

        assert_eq!(service.state(), &State::Active);
        assert_eq!(effect_names(&result), vec![Effect::ResumeAllTimers]);
    }

    #[test]
    fn set_props_emits_sync_props_when_context_relevant_fields_change() {
        let mut service = fresh_service(test_props());

        // Add a toast so we can verify the toast list is preserved.
        drop(service.send(Event::Add(add_config(Kind::Info, "live"))));

        let toast_count_before = service.context().toasts.len();

        let result = service.set_props(Props {
            id: "toaster".to_string(),
            placement: Placement::TopCenter,
            max_visible: 3,
            gap: 8.0,
            remove_delay: Duration::from_millis(100),
            overlap: true,
            ..Props::default()
        });

        // SyncProps event was synthesized and applied — context now mirrors
        // new props, but the live toast list is preserved.
        assert!(
            result.context_changed,
            "set_props with relevant changes must update context"
        );

        let ctx = service.context();

        assert_eq!(ctx.placement, Placement::TopCenter);
        assert_eq!(ctx.max_visible, 3);
        assert_eq!(ctx.gap, 8.0);
        assert_eq!(ctx.remove_delay, Duration::from_millis(100));
        assert!(ctx.overlap);
        assert_eq!(ctx.toasts.len(), toast_count_before);
    }

    #[test]
    fn set_props_with_no_relevant_changes_is_noop() {
        let mut service = fresh_service(test_props());

        let result = service.set_props(test_props());

        assert!(!result.context_changed);
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn sync_props_max_visible_shrink_preserves_existing_toasts() {
        // Admit two toasts under max_visible=2.
        let mut service = fresh_service(Props {
            max_visible: 2,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "b"))));

        assert_eq!(service.context().toasts.len(), 2);

        // Shrink max_visible to 1 mid-flight.
        drop(service.set_props(Props {
            id: "toaster".to_string(),
            max_visible: 1,
            ..Props::default()
        }));

        // Both toasts MUST still be visible — `max_visible` is an
        // admission cap, not a retroactive cull (see spec §2.6).
        assert_eq!(service.context().max_visible, 1);
        assert_eq!(service.context().toasts.len(), 2);

        // The next Add now queues because we're already over the new cap.
        drop(service.send(Event::Add(add_config(Kind::Info, "c"))));

        assert_eq!(service.context().toasts.len(), 2);
        assert_eq!(service.context().queued.len(), 1);
    }

    #[test]
    fn sync_props_clamps_max_visible_to_one() {
        let mut service = fresh_service(test_props());

        let mut new_props = test_props();

        new_props.max_visible = 0;

        drop(service.set_props(new_props));

        assert_eq!(service.context().max_visible, 1);
    }

    #[test]
    fn set_visibility_true_when_already_active_is_noop() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::SetVisibility(true));

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
    }

    // ── Cross-pause reason interactions ────────────────────────────

    #[test]
    fn visibility_cycle_during_interaction_pause_keeps_pause_alive() {
        // Regression test for the round-5 review finding: pause source
        // tracking. Hover/focus pauses; tab hides and re-shows; the
        // toast must remain paused because the user is still
        // interacting.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::PauseAll));
        assert_eq!(service.state(), &State::Paused);
        assert!(service.context().pause_reasons.interaction);
        assert!(!service.context().pause_reasons.visibility);

        // Tab hides while still hovered. We're already paused → no
        // extra `PauseAllTimers` (timers are already stopped); we just
        // arm the visibility flag.
        let hide = service.send(Event::SetVisibility(false));
        assert_eq!(service.state(), &State::Paused);
        assert!(service.context().pause_reasons.interaction);
        assert!(service.context().pause_reasons.visibility);
        assert!(!hide.state_changed);
        assert!(hide.pending_effects.is_empty());

        // Tab shows again — but interaction pause is still active. The
        // manager must NOT unpause here: that would auto-dismiss
        // toasts the user is still reading. Only the visibility flag
        // clears; no `ResumeAllTimers` effect.
        let show = service.send(Event::SetVisibility(true));
        assert_eq!(
            service.state(),
            &State::Paused,
            "interaction pause survives a tab hide/show cycle"
        );
        assert!(service.context().pause_reasons.interaction);
        assert!(!service.context().pause_reasons.visibility);
        assert!(service.context().paused_all);
        assert!(!show.state_changed);
        assert!(show.pending_effects.is_empty());

        // Releasing the hover/focus is now the *last* reason; only
        // here do timers actually resume.
        let resume = service.send(Event::ResumeAll);
        assert_eq!(service.state(), &State::Active);
        assert!(!service.context().pause_reasons.any());
        assert!(!service.context().paused_all);
        assert_eq!(effect_names(&resume), vec![Effect::ResumeAllTimers]);
    }

    #[test]
    fn interaction_pause_during_visibility_pause_keeps_pause_alive() {
        // Symmetric case: tab is hidden first, user hovers a toast
        // before the tab reappears (rare but possible — e.g. with a
        // background tab brought back via keyboard shortcut over a
        // sticky toast). Tab show alone must not resume.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetVisibility(false)));
        assert!(service.context().pause_reasons.visibility);
        assert!(!service.context().pause_reasons.interaction);

        // Interaction layered on top while still hidden — already
        // paused, no extra `PauseAllTimers` effect.
        let hover = service.send(Event::PauseAll);
        assert_eq!(service.state(), &State::Paused);
        assert!(service.context().pause_reasons.interaction);
        assert!(service.context().pause_reasons.visibility);
        assert!(!hover.state_changed);
        assert!(hover.pending_effects.is_empty());

        // Tab visible again, but hover still active → stay paused,
        // no resume effect.
        let show = service.send(Event::SetVisibility(true));
        assert_eq!(service.state(), &State::Paused);
        assert!(service.context().pause_reasons.interaction);
        assert!(!service.context().pause_reasons.visibility);
        assert!(service.context().paused_all);
        assert!(!show.state_changed);
        assert!(show.pending_effects.is_empty());

        // Hover releases → resume.
        let resume = service.send(Event::ResumeAll);
        assert_eq!(service.state(), &State::Active);
        assert!(!service.context().paused_all);
        assert_eq!(effect_names(&resume), vec![Effect::ResumeAllTimers]);
    }

    #[test]
    fn pause_all_when_already_visibility_paused_arms_interaction_flag() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetVisibility(false)));
        assert!(!service.context().pause_reasons.interaction);

        let result = service.send(Event::PauseAll);

        // Already paused — no transition, no extra effect, but the
        // interaction reason is now armed.
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert!(service.context().pause_reasons.interaction);
        assert!(service.context().pause_reasons.visibility);
    }

    #[test]
    fn set_visibility_false_when_already_interaction_paused_arms_flag() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::PauseAll));
        assert!(!service.context().pause_reasons.visibility);

        let result = service.send(Event::SetVisibility(false));

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert!(service.context().pause_reasons.interaction);
        assert!(service.context().pause_reasons.visibility);
    }

    #[test]
    fn resume_all_without_interaction_reason_is_noop_even_when_visibility_paused() {
        // Adapter sends a stray `ResumeAll` (e.g. the user
        // double-tapped Escape) while only the page-visibility pause
        // is active. The interaction reason is not set, so there is
        // nothing to clear and the manager must stay paused.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetVisibility(false)));
        let snapshot = service.context().pause_reasons;

        let result = service.send(Event::ResumeAll);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::Paused);
        assert_eq!(service.context().pause_reasons, snapshot);
    }

    #[test]
    fn set_visibility_true_without_visibility_reason_is_noop_when_interaction_paused() {
        // Symmetric stray-event case: spurious `SetVisibility(true)`
        // while only interaction-paused — visibility flag was never
        // set, so the call is a noop and timers stay paused.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::PauseAll));
        let snapshot = service.context().pause_reasons;

        let result = service.send(Event::SetVisibility(true));

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::Paused);
        assert_eq!(service.context().pause_reasons, snapshot);
    }

    #[test]
    fn visibility_pause_then_show_when_no_interaction_resumes() {
        // Pure-visibility happy path: tab hidden then shown with no
        // interaction pause overlapping → full resume.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetVisibility(false)));
        assert_eq!(service.state(), &State::Paused);

        let show = service.send(Event::SetVisibility(true));

        assert_eq!(service.state(), &State::Active);
        assert!(!service.context().pause_reasons.any());
        assert_eq!(effect_names(&show), vec![Effect::ResumeAllTimers]);
    }

    #[test]
    fn pause_reasons_default_is_no_pause() {
        let reasons = PauseReasons::default();
        assert!(!reasons.any());
        assert!(!reasons.interaction);
        assert!(!reasons.visibility);
    }

    #[test]
    fn resume_all_while_both_paused_clears_interaction_only() {
        // Both reasons active. ResumeAll clears interaction but
        // visibility-pause keeps the manager paused. No
        // `ResumeAllTimers` effect (timers must stay paused).
        let mut service = fresh_service(test_props());

        drop(service.send(Event::PauseAll));
        drop(service.send(Event::SetVisibility(false)));
        assert!(service.context().pause_reasons.interaction);
        assert!(service.context().pause_reasons.visibility);

        let result = service.send(Event::ResumeAll);

        assert_eq!(service.state(), &State::Paused);
        assert!(!service.context().pause_reasons.interaction);
        assert!(service.context().pause_reasons.visibility);
        assert!(service.context().paused_all);
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn set_visibility_false_when_already_visibility_paused_is_noop() {
        // Adapter sends a redundant `SetVisibility(false)` (e.g. two
        // `visibilitychange` events in quick succession). Visibility
        // flag is already set; nothing to update.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetVisibility(false)));
        let snapshot = service.context().pause_reasons;

        let result = service.send(Event::SetVisibility(false));

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.context().pause_reasons, snapshot);
    }

    // ── Announcement queue / drain ─────────────────────────────────

    #[test]
    fn drain_announcement_assertive_drains_before_polite() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "polite"))));
        drop(service.send(Event::Add(add_config(Kind::Error, "assertive"))));

        // Two announcements queued.
        assert_eq!(service.context().announcement_queue.len(), 2);

        let result = service.send(Event::DrainAnnouncement { now_ms: 0 });

        assert!(effect_names(&result).contains(&Effect::AnnounceAssertive));
        assert_eq!(service.context().announcement_queue.len(), 1);

        // Bump the clock past the 500 ms gap.
        let result = service.send(Event::DrainAnnouncement { now_ms: 750 });

        assert!(effect_names(&result).contains(&Effect::AnnouncePolite));
        assert!(service.context().announcement_queue.is_empty());
    }

    #[test]
    fn drain_announcement_respects_500ms_gap() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "b"))));

        // First drain at t=0 succeeds.
        drop(service.send(Event::DrainAnnouncement { now_ms: 0 }));

        assert_eq!(service.context().announcement_queue.len(), 1);

        // Second drain at t=200ms is blocked.
        let result = service.send(Event::DrainAnnouncement { now_ms: 200 });

        assert!(result.pending_effects.is_empty());
        assert_eq!(service.context().announcement_queue.len(), 1);

        // At t=500ms it succeeds.
        let result = service.send(Event::DrainAnnouncement { now_ms: 500 });

        assert!(!result.pending_effects.is_empty());
        assert!(service.context().announcement_queue.is_empty());
    }

    #[test]
    fn drain_announcement_with_empty_queue_is_noop() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::DrainAnnouncement { now_ms: 0 });

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn drain_announcement_keeps_polite_fifo_within_priority() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "first"))));
        drop(service.send(Event::Add(add_config(Kind::Success, "second"))));

        let mut drained = Vec::new();

        let mut now = 0_u64;

        for _ in 0..2 {
            let result = service.send(Event::DrainAnnouncement { now_ms: now });

            for effect in result.pending_effects {
                drained.push(effect.name);
            }

            now += 500;
        }

        // Both announcements must drain in insertion order; both are polite.
        let polite_count = drained
            .iter()
            .filter(|e| matches!(e, Effect::AnnouncePolite))
            .count();

        assert!(polite_count >= 2);
    }

    // ── Toaster / Promise builders ──────────────────────────────────

    #[test]
    fn toaster_intent_helpers_build_configs() {
        for (built, expected_kind) in [
            (Toaster::info("Hi", "info-body"), Kind::Info),
            (Toaster::success("Hi", "success-body"), Kind::Success),
            (Toaster::warning("Hi", "warning-body"), Kind::Warning),
            (Toaster::error("Hi", "error-body"), Kind::Error),
        ] {
            assert_eq!(built.kind, expected_kind);
            assert_eq!(built.title.as_deref(), Some("Hi"));
            assert!(built.description.is_some());
        }

        let loading = Toaster::loading("Saving", "...");

        assert_eq!(loading.kind, Kind::Loading);
        assert!(
            loading.duration.is_none(),
            "loading toasts default to persistent (duration: None)"
        );
    }

    #[test]
    fn toast_content_builder_round_trips() {
        let content = ToastContent::new("Saving").description("uploading file");

        assert_eq!(content.title.as_deref(), Some("Saving"));
        assert_eq!(content.description.as_deref(), Some("uploading file"));
    }

    #[test]
    fn promise_clone_and_debug_round_trip() {
        let p: Promise<i32, &'static str> = Promise::new(
            ToastContent::new("loading"),
            |n: i32| ToastContent::new(format!("ok {n}")),
            |e: &'static str| ToastContent::new(format!("err {e}")),
        );

        let cloned = p.clone();

        // Cloned mappers still produce the same outputs (Arc-shared).
        assert_eq!((cloned.success)(7).title.as_deref(), Some("ok 7"));
        assert_eq!((cloned.error)("x").title.as_deref(), Some("err x"));
        assert_eq!(cloned.loading.title.as_deref(), Some("loading"));

        // Debug impl elides callbacks but renders loading body.
        let dbg = format!("{p:?}");

        assert!(dbg.contains("Promise"));
        assert!(dbg.contains("loading"));
        assert!(dbg.contains("<callback>"));
    }

    #[test]
    fn manager_api_debug_does_not_panic() {
        let service = fresh_service(test_props());

        let api = service.connect(&|_| {});

        let dbg = format!("{api:?}");

        assert!(dbg.contains("Api"));
    }

    #[test]
    fn hide_queue_advance_with_unknown_id_at_capacity_takes_else_branch() {
        // max_visible=1, one visible toast, empty queue. Issuing
        // HideQueueAdvance for an id that doesn't match anything in
        // ctx.toasts means `visible_after_remove == 1 == max_visible`,
        // so `promote_kind` takes the `else { None }` branch. The
        // closure's retain is also a no-op. The result is a context-only
        // plan with no announcement effect.
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "live"))));

        let toasts_before = service.context().toasts.clone();

        let result = service.send(Event::HideQueueAdvance("unknown".to_string()));

        // No promotion happened; live toast unchanged.
        assert_eq!(service.context().toasts, toasts_before);
        assert!(result.pending_effects.is_empty());
    }

    /// `Update` no longer leaves `duration: None` on a queued slot —
    /// `plan_update` applies `DefaultDurations::for_kind` before writing
    /// to the slot (per the P1.A review fix). This test pins the new
    /// contract: even an `Update` whose builder calls `.duration(None)`
    /// does NOT leak a None duration through the queue. (The previous
    /// contract relied on a promotion-time fallback in
    /// `plan_hide_queue_advance` that's now genuinely unreachable.)
    #[test]
    fn update_on_queued_slot_fills_default_duration() {
        let mut service = fresh_service(Props {
            max_visible: 1,
            ..test_props()
        });

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "b"))));

        let queued_id = service.context().queued[0].id.clone().unwrap();

        // Update the queued slot with explicit `duration: None`. Before
        // the P1.A fix this would leak a None duration into the queue,
        // breaking the auto-dismiss contract for the toast whenever it
        // promotes. After the fix `plan_update` applies the default
        // before writing the slot.
        drop(
            service.send(Event::Update(
                queued_id.clone(),
                Config::new(Kind::Info, "b-updated")
                    .description("body")
                    .duration(None),
            )),
        );

        // Queue holds a fully-resolved config — no None leak.
        assert_eq!(
            service.context().queued[0].duration,
            Some(Duration::from_secs(5)),
            "Update must fill the default before writing to the queue"
        );
        assert_eq!(
            service.context().queued[0].title.as_deref(),
            Some("b-updated")
        );

        // After promotion the toast carries the same filled duration.
        let live_id = service.context().toasts[0].id.clone();

        drop(service.send(Event::Remove(live_id.clone())));
        drop(service.send(Event::HideQueueAdvance(live_id)));

        let promoted = &service.context().toasts[0];

        assert_eq!(promoted.config.duration, Some(Duration::from_secs(5)));
        assert_eq!(promoted.config.title.as_deref(), Some("b-updated"));
    }

    #[test]
    fn promise_carries_loading_and_invokes_mappers() {
        let promise: Promise<i32, &'static str> = Promise::new(
            ToastContent::new("Saving"),
            |n: i32| ToastContent::new(format!("Saved {n}")),
            |e: &'static str| ToastContent::new(format!("Failed: {e}")),
        );

        assert_eq!(promise.loading.title.as_deref(), Some("Saving"));
        assert_eq!((promise.success)(42).title.as_deref(), Some("Saved 42"));
        assert_eq!(
            (promise.error)("oops").title.as_deref(),
            Some("Failed: oops")
        );
    }

    // ── Api / connect ──────────────────────────────────────────────

    #[test]
    fn api_dispatch_helpers_send_expected_events() {
        let service = fresh_service(test_props());
        let sent = Rc::new(RefCell::new(Vec::new()));
        let sent_clone = Rc::clone(&sent);

        let send = move |event| sent_clone.borrow_mut().push(event);

        let api = service.connect(&send);

        api.add(Config::new(Kind::Info, "hello"));
        api.update("toast-1", Config::new(Kind::Success, "done"));
        api.dismiss("toast-1");
        api.dismiss_all();
        api.pause_all();
        api.resume_all();
        api.drain_announcement(123);

        let events = sent.borrow();

        assert_eq!(events.len(), 7);
        assert!(matches!(events[0], Event::Add(_)));
        assert!(matches!(events[1], Event::Update(_, _)));
        assert!(matches!(events[2], Event::Remove(_)));
        assert!(matches!(events[3], Event::DismissAll));
        assert!(matches!(events[4], Event::PauseAll));
        assert!(matches!(events[5], Event::ResumeAll));
        assert!(
            matches!(events[6], Event::DrainAnnouncement { now_ms: 123 }),
            "expected DrainAnnouncement with now_ms=123, got {:?}",
            events[6]
        );
    }

    #[test]
    fn api_visible_ids_filter_by_stage() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Add(add_config(Kind::Info, "a"))));
        drop(service.send(Event::Add(add_config(Kind::Info, "b"))));

        let id = service.context().toasts[0].id.clone();

        drop(service.send(Event::Remove(id.clone())));

        let api = service.connect(&|_| {});
        let visible = api.visible_ids();

        assert_eq!(visible.len(), 1);
        assert_ne!(visible[0], id);
    }

    // ── Snapshots ───────────────────────────────────────────────────

    #[test]
    fn snapshot_root_default_placement() {
        let service = fresh_service(test_props());

        let api = service.connect(&|_| {});

        assert_snapshot!("manager_root_bottom_end", snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn snapshot_root_top_center_overlap() {
        let service = fresh_service(Props {
            placement: Placement::TopCenter,
            overlap: true,
            ..test_props()
        });

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "manager_root_top_center_overlap",
            snapshot_attrs(&api.root_attrs())
        );
    }

    #[test]
    fn snapshot_root_paused() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::PauseAll));

        let api = service.connect(&|_| {});

        assert_snapshot!("manager_root_paused", snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn snapshot_polite_region() {
        let service = fresh_service(test_props());

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "manager_polite_region",
            snapshot_attrs(&api.region_attrs(false))
        );
    }

    #[test]
    fn snapshot_assertive_region() {
        let service = fresh_service(test_props());

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "manager_assertive_region",
            snapshot_attrs(&api.region_attrs(true))
        );
    }

    #[test]
    fn snapshot_region_helper_polite_localized() {
        let messages = Messages {
            region_label: MessageFn::static_str("Notificaciones"),
        };

        let env = Env::default();

        assert_snapshot!(
            "region_helper_polite_localized",
            snapshot_attrs(&region_attrs(&messages, &env.locale, RegionPart::Polite))
        );
    }

    #[test]
    fn snapshot_region_helper_assertive() {
        let messages = Messages::default();

        let env = Env::default();

        assert_snapshot!(
            "region_helper_assertive",
            snapshot_attrs(&region_attrs(&messages, &env.locale, RegionPart::Assertive))
        );
    }
}
