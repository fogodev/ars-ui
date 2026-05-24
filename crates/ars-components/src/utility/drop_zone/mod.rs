//! DropZone component state machine and connect API.

use alloc::{borrow::ToOwned, string::String, vec, vec::Vec};
use core::{
    fmt::{self, Debug, Display},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan, no_cleanup,
};
use ars_interactions::{DragItem, DropOperation};

/// Callback shape for resolving a drop operation from payload data and allowed operations.
pub type DropOperationFn = dyn Fn((DragData, Vec<DropOperation>)) -> DropOperation + Send + Sync;

/// Callback shape for drag payload notifications.
pub type DragDataFn = dyn Fn(DragData) + Send + Sync;

/// Callback shape for accepted drop items.
pub type DragItemsFn = dyn Fn(Vec<DragItem>) + Send + Sync;

/// Callback shape for rejected drop details.
pub type DropRejectionFn = dyn Fn(DropRejection) + Send + Sync;

/// Callback shape for zero-argument `DropZone` notifications.
pub type VoidFn = dyn Fn() + Send + Sync;

/// The states for the `DropZone` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Default resting state with no active drag interaction.
    Idle,

    /// A drag operation is hovering over the drop zone.
    DragOver,

    /// The latest drop was accepted.
    DropAccepted,

    /// The latest drop was rejected by validation.
    DropRejected,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::DragOver => f.write_str("drag-over"),
            Self::DropAccepted => f.write_str("drop-accepted"),
            Self::DropRejected => f.write_str("drop-rejected"),
        }
    }
}

/// Data associated with a drag operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragData {
    /// Fully resolved dragged items.
    ///
    /// This may be empty before the final drop because browsers often expose
    /// only advertised MIME types during hover.
    pub items: Vec<DragItem>,

    /// MIME types advertised by the drag source.
    pub types: Vec<String>,
}

/// Validation failures collected while evaluating a drop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropValidationError {
    /// A payload MIME type is outside [`Props::accept`].
    UnsupportedType {
        /// The rejected MIME type after `DropZone` normalization.
        mime_type: String,
    },

    /// The payload contains more items than [`Props::max_files`] allows.
    TooManyFiles {
        /// Number of dropped items.
        actual: usize,

        /// Maximum allowed item count.
        max: usize,
    },

    /// A dropped file exceeds [`Props::max_file_size`].
    FileTooLarge {
        /// Display name of the oversized file.
        name: String,

        /// Actual file size in bytes.
        size: u64,

        /// Maximum allowed file size in bytes.
        max: u64,
    },
}

/// Structured details passed to rejected-drop callbacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropRejection {
    /// The rejected drag payload.
    pub data: DragData,

    /// Every validation failure found for the payload.
    pub errors: Vec<DropValidationError>,
}

/// Events for the `DropZone` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A drag operation entered the drop zone.
    DragEnter(DragData),

    /// A drag operation is hovering over the drop zone.
    DragOver(DragData),

    /// The drag operation left the drop zone.
    DragLeave,

    /// Items were dropped onto the drop zone.
    Drop(DragData),

    /// Clear transient state and return to idle.
    Reset,

    /// Synchronize context fields from updated props.
    SetProps,

    /// The delayed drop-activation timer fired while a drag hovered.
    DropActivate,

    /// The drop zone received focus.
    Focus {
        /// Whether focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// The drop zone lost focus.
    Blur,
}

/// Named effect intents emitted by the `DropZone` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// A drag operation entered the drop target.
    DropEnter,

    /// A drag operation left the drop target.
    DropExit,

    /// A drag operation moved while hovering the drop target.
    DropMove,

    /// Adapter should start the delayed drop-activation timer.
    ArmDropActivate,

    /// The delayed drop-activation timer fired.
    DropActivate,

    /// A drop was accepted.
    DropAccepted,

    /// A drop was rejected.
    DropRejected,

    /// Adapter should return terminal accepted/rejected state to idle after
    /// [`Props::reset_delay`].
    ResetAfterDrop,
}

/// Context for the `DropZone` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// MIME types accepted by this drop zone.
    pub accept: Vec<String>,

    /// Maximum number of items accepted by this drop zone.
    pub max_files: Option<usize>,

    /// Maximum accepted file size in bytes.
    pub max_file_size: Option<u64>,

    /// Whether drop interactions are disabled.
    pub disabled: bool,

    /// Whether the root currently has focus.
    pub focused: bool,

    /// Whether focus should render as keyboard-visible focus.
    pub focus_visible: bool,

    /// Whether the active drag currently satisfies advertised type policy.
    pub valid_drag: bool,

    /// Whether a drag operation is currently hovering over the drop target.
    pub is_drop_target: bool,

    /// Last accepted dropped items.
    pub dropped_items: Vec<DragItem>,

    /// Last rejected payload and validation errors.
    pub last_rejection: Option<DropRejection>,

    /// Stable IDs for semantic relationships and adapter wiring.
    pub ids: ComponentIds,

    /// Resolved locale for messages.
    pub locale: Locale,

    /// Localizable message bundle.
    pub messages: Messages,

    /// Whether the drop zone is read-only.
    pub read_only: bool,
}

/// Props for the `DropZone` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// MIME types to accept. Empty means accept all types.
    pub accept: Vec<String>,

    /// Maximum number of items that can be dropped.
    pub max_files: Option<usize>,

    /// Maximum file size in bytes.
    pub max_file_size: Option<u64>,

    /// Whether drop interactions are disabled.
    pub disabled: bool,

    /// Accessible label. Falls back to [`Messages::label`] when empty.
    pub label: String,

    /// Drop operations this target accepts.
    pub allowed_operations: Vec<DropOperation>,

    /// Form field name used by adapter submit handlers.
    pub name: Option<String>,

    /// Whether a value is required for form validation.
    pub required: bool,

    /// Whether external form validation marks the field invalid.
    pub invalid: bool,

    /// Whether the drop zone prevents new drops while keeping form data readable.
    pub read_only: bool,

    /// Delay before firing [`Event::DropActivate`].
    pub activate_delay: Duration,

    /// Delay before adapters reset terminal drop state to idle.
    pub reset_delay: Duration,

    /// Optional callback used by adapters to resolve an operation for a drag.
    pub get_drop_operation: Option<Callback<DropOperationFn>>,

    /// Callback fired when a valid drop is accepted.
    pub on_drop: Option<Callback<DragItemsFn>>,

    /// Callback fired when a drop is rejected.
    pub on_drop_rejected: Option<Callback<DropRejectionFn>>,

    /// Callback fired when a drag enters the zone.
    pub on_drop_enter: Option<Callback<DragDataFn>>,

    /// Callback fired when a drag leaves the zone.
    pub on_drop_exit: Option<Callback<VoidFn>>,

    /// Callback fired while a drag moves over the zone.
    pub on_drop_move: Option<Callback<DragDataFn>>,

    /// Callback fired when pointer hover starts.
    pub on_hover_start: Option<Callback<VoidFn>>,

    /// Callback fired after the activation delay elapses.
    pub on_drop_activate: Option<Callback<VoidFn>>,

    /// Callback fired when pointer hover ends.
    pub on_hover_end: Option<Callback<VoidFn>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            accept: Vec::new(),
            max_files: None,
            max_file_size: None,
            disabled: false,
            label: String::new(),
            allowed_operations: vec![DropOperation::Move],
            name: None,
            required: false,
            invalid: false,
            read_only: false,
            activate_delay: Duration::from_millis(500),
            reset_delay: Duration::from_millis(1_500),
            get_drop_operation: None,
            on_drop: None,
            on_drop_rejected: None,
            on_drop_enter: None,
            on_drop_exit: None,
            on_drop_move: None,
            on_hover_start: None,
            on_drop_activate: None,
            on_hover_end: None,
        }
    }
}

impl Props {
    /// Returns default `DropZone` props.
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

    /// Sets [`accept`](Self::accept).
    #[must_use]
    pub fn accept<I, S>(mut self, accept: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.accept = accept.into_iter().map(Into::into).collect();
        self
    }

    /// Sets [`max_files`](Self::max_files).
    #[must_use]
    pub const fn max_files(mut self, max_files: usize) -> Self {
        self.max_files = Some(max_files);
        self
    }

    /// Clears [`max_files`](Self::max_files).
    #[must_use]
    pub const fn clear_max_files(mut self) -> Self {
        self.max_files = None;
        self
    }

    /// Sets [`max_file_size`](Self::max_file_size).
    #[must_use]
    pub const fn max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = Some(max_file_size);
        self
    }

    /// Clears [`max_file_size`](Self::max_file_size).
    #[must_use]
    pub const fn clear_max_file_size(mut self) -> Self {
        self.max_file_size = None;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`label`](Self::label).
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets [`allowed_operations`](Self::allowed_operations).
    #[must_use]
    pub fn allowed_operations<I>(mut self, allowed_operations: I) -> Self
    where
        I: IntoIterator<Item = DropOperation>,
    {
        self.allowed_operations = allowed_operations.into_iter().collect();
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Clears [`name`](Self::name).
    #[must_use]
    pub fn clear_name(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets [`read_only`](Self::read_only).
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Sets [`activate_delay`](Self::activate_delay).
    #[must_use]
    pub const fn activate_delay(mut self, activate_delay: Duration) -> Self {
        self.activate_delay = activate_delay;
        self
    }

    /// Sets [`reset_delay`](Self::reset_delay).
    #[must_use]
    pub const fn reset_delay(mut self, reset_delay: Duration) -> Self {
        self.reset_delay = reset_delay;
        self
    }

    /// Sets [`get_drop_operation`](Self::get_drop_operation).
    #[must_use]
    pub fn get_drop_operation(mut self, callback: impl Into<Callback<DropOperationFn>>) -> Self {
        self.get_drop_operation = Some(callback.into());
        self
    }

    /// Sets [`on_drop`](Self::on_drop).
    #[must_use]
    pub fn on_drop(mut self, callback: impl Into<Callback<DragItemsFn>>) -> Self {
        self.on_drop = Some(callback.into());
        self
    }

    /// Sets [`on_drop_rejected`](Self::on_drop_rejected).
    #[must_use]
    pub fn on_drop_rejected(mut self, callback: impl Into<Callback<DropRejectionFn>>) -> Self {
        self.on_drop_rejected = Some(callback.into());
        self
    }

    /// Sets [`on_drop_enter`](Self::on_drop_enter).
    #[must_use]
    pub fn on_drop_enter(mut self, callback: impl Into<Callback<DragDataFn>>) -> Self {
        self.on_drop_enter = Some(callback.into());
        self
    }

    /// Sets [`on_drop_exit`](Self::on_drop_exit).
    #[must_use]
    pub fn on_drop_exit(mut self, callback: impl Into<Callback<VoidFn>>) -> Self {
        self.on_drop_exit = Some(callback.into());
        self
    }

    /// Sets [`on_drop_move`](Self::on_drop_move).
    #[must_use]
    pub fn on_drop_move(mut self, callback: impl Into<Callback<DragDataFn>>) -> Self {
        self.on_drop_move = Some(callback.into());
        self
    }

    /// Sets [`on_hover_start`](Self::on_hover_start).
    #[must_use]
    pub fn on_hover_start(mut self, callback: impl Into<Callback<VoidFn>>) -> Self {
        self.on_hover_start = Some(callback.into());
        self
    }

    /// Sets [`on_drop_activate`](Self::on_drop_activate).
    #[must_use]
    pub fn on_drop_activate(mut self, callback: impl Into<Callback<VoidFn>>) -> Self {
        self.on_drop_activate = Some(callback.into());
        self
    }

    /// Sets [`on_hover_end`](Self::on_hover_end).
    #[must_use]
    pub fn on_hover_end(mut self, callback: impl Into<Callback<VoidFn>>) -> Self {
        self.on_hover_end = Some(callback.into());
        self
    }
}

/// Localizable strings for the `DropZone` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Default accessible label when [`Props::label`] is empty.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Description announced while a valid drag hovers over the zone.
    pub drop_ready_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Announcement when a drop is accepted.
    pub drop_accepted_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Announcement when a drop is rejected.
    pub drop_rejected_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Drop files here"),
            drop_ready_description: MessageFn::static_str("Release to drop files"),
            drop_accepted_announcement: MessageFn::static_str("Files accepted"),
            drop_rejected_announcement: MessageFn::static_str("Files rejected"),
        }
    }
}

impl ComponentMessages for Messages {}

/// The machine for the `DropZone` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
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
            State::Idle,
            Context {
                accept: props.accept.clone(),
                max_files: props.max_files,
                max_file_size: props.max_file_size,
                disabled: props.disabled,
                focused: false,
                focus_visible: false,
                valid_drag: false,
                is_drop_target: false,
                dropped_items: Vec::new(),
                last_rejection: None,
                ids: ComponentIds::from_id(&props.id),
                locale: env.locale.clone(),
                messages: messages.clone(),
                read_only: props.read_only,
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled
            && !matches!(
                event,
                Event::Focus { .. } | Event::Blur | Event::SetProps | Event::Reset
            )
        {
            return None;
        }

        if ctx.read_only
            && matches!(
                event,
                Event::DragEnter(_)
                    | Event::DragOver(_)
                    | Event::DragLeave
                    | Event::Drop(_)
                    | Event::DropActivate
            )
        {
            return None;
        }

        match (state, event) {
            (_, Event::SetProps) => {
                let accept = props.accept.clone();
                let max_files = props.max_files;
                let max_file_size = props.max_file_size;
                let disabled = props.disabled;
                let read_only = props.read_only;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.accept = accept;
                    ctx.max_files = max_files;
                    ctx.max_file_size = max_file_size;
                    ctx.disabled = disabled;
                    ctx.read_only = read_only;
                }))
            }

            (State::Idle | State::DropAccepted | State::DropRejected, Event::DragEnter(data)) => {
                let data = data.clone();
                let valid = validate_drag_types(ctx, &data.types);

                Some(
                    TransitionPlan::to(State::DragOver)
                        .apply(move |ctx: &mut Context| {
                            ctx.valid_drag = valid;
                            ctx.is_drop_target = true;

                            ctx.dropped_items.clear();
                            ctx.last_rejection = None;
                        })
                        .with_effect(drop_enter_effect(data))
                        .with_effect(PendingEffect::named(Effect::ArmDropActivate)),
                )
            }

            (State::DragOver, Event::DragOver(data)) => {
                let data = data.clone();
                let valid = validate_drag_types(ctx, &data.types);

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.valid_drag = valid;
                    })
                    .with_effect(drop_move_effect(data)),
                )
            }

            (State::DragOver, Event::DropActivate) => {
                Some(TransitionPlan::new().with_effect(drop_activate_effect()))
            }

            (State::DragOver, Event::DragLeave) => Some(
                TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        ctx.valid_drag = false;
                        ctx.is_drop_target = false;
                    })
                    .with_effect(drop_exit_effect())
                    .cancel_effect(Effect::ArmDropActivate),
            ),

            (State::DragOver, Event::Drop(data)) => {
                let errors = validate_drop(ctx, &data.items);
                if errors.is_empty() {
                    let items = data.items.clone();
                    Some(
                        TransitionPlan::to(State::DropAccepted)
                            .apply({
                                let items = items.clone();
                                move |ctx: &mut Context| {
                                    ctx.dropped_items = items;
                                    ctx.last_rejection = None;
                                    ctx.valid_drag = false;
                                    ctx.is_drop_target = false;
                                }
                            })
                            .with_effect(drop_accepted_effect(items))
                            .with_effect(PendingEffect::named(Effect::ResetAfterDrop))
                            .cancel_effect(Effect::ArmDropActivate),
                    )
                } else {
                    let rejection = DropRejection {
                        data: data.clone(),
                        errors,
                    };
                    Some(
                        TransitionPlan::to(State::DropRejected)
                            .apply({
                                let rejection = rejection.clone();
                                move |ctx: &mut Context| {
                                    ctx.last_rejection = Some(rejection);
                                    ctx.valid_drag = false;
                                    ctx.is_drop_target = false;
                                }
                            })
                            .with_effect(drop_rejected_effect(rejection))
                            .with_effect(PendingEffect::named(Effect::ResetAfterDrop))
                            .cancel_effect(Effect::ArmDropActivate),
                    )
                }
            }

            (State::DragOver | State::DropAccepted | State::DropRejected, Event::Reset) => {
                Some(reset_plan())
            }

            (_, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (_, Event::Blur) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            _ => None,
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
        assert_eq!(
            old.id, new.id,
            "drop_zone::Props.id must remain stable after init",
        );

        if old.accept != new.accept
            || old.max_files != new.max_files
            || old.max_file_size != new.max_file_size
            || old.disabled != new.disabled
            || old.read_only != new.read_only
        {
            vec![Event::SetProps]
        } else {
            Vec::new()
        }
    }
}

/// DOM parts of the `DropZone` component.
#[derive(ComponentPart)]
#[scope = "drop-zone"]
pub enum Part {
    /// The root drop target element.
    Root,
}

/// The API for the `DropZone` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .field("send", &"<callback>")
            .finish()
    }
}

impl Api<'_> {
    /// Returns HTML attributes for the root drop target element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), self.state.to_string())
            .set(HtmlAttr::Role, "button")
            .set(HtmlAttr::TabIndex, "0");

        if self.props.label.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            );
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.props.label.as_str());
        }

        if matches!(self.state, State::DragOver) {
            attrs.set_bool(HtmlAttr::Data("ars-drag-over"), true);
        }

        if matches!(self.state, State::DragOver) && self.ctx.valid_drag {
            attrs
                .set(
                    HtmlAttr::Aria(AriaAttr::Description),
                    (self.ctx.messages.drop_ready_description)(&self.ctx.locale),
                )
                .set_bool(HtmlAttr::Data("ars-drop-ready"), true);
        }

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.read_only {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.props.invalid || matches!(self.state, State::DropRejected) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.ctx.valid_drag {
            attrs.set_bool(HtmlAttr::Data("ars-drag-valid"), true);
        }

        attrs
    }

    /// Returns dropped items for adapter form submission.
    #[must_use]
    pub fn form_data(&self) -> &[DragItem] {
        if self.ctx.disabled || self.props.name.is_none() {
            &[]
        } else {
            &self.ctx.dropped_items
        }
    }

    /// Builds the localized announcement for an accepted drop.
    ///
    /// Adapters pass this string to their live-region announcer when the
    /// machine reaches [`State::DropAccepted`].
    #[must_use]
    pub fn drop_accepted_announcement(&self) -> String {
        (self.ctx.messages.drop_accepted_announcement)(&self.ctx.locale)
    }

    /// Builds the localized announcement for a rejected drop.
    ///
    /// Adapters pass this string to their live-region announcer when the
    /// machine reaches [`State::DropRejected`].
    #[must_use]
    pub fn drop_rejected_announcement(&self) -> String {
        (self.ctx.messages.drop_rejected_announcement)(&self.ctx.locale)
    }

    /// Dispatches a focus event.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches a blur event.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches a drag-enter event.
    pub fn on_drag_enter(&self, data: DragData) {
        (self.send)(Event::DragEnter(data));
    }

    /// Dispatches a drag-over event.
    pub fn on_drag_over(&self, data: DragData) {
        (self.send)(Event::DragOver(data));
    }

    /// Dispatches a drag-leave event.
    pub fn on_drag_leave(&self) {
        (self.send)(Event::DragLeave);
    }

    /// Dispatches a drop event.
    pub fn on_drop(&self, data: DragData) {
        (self.send)(Event::Drop(data));
    }

    /// Dispatches a form-reset event.
    pub fn on_form_reset(&self) {
        (self.send)(Event::Reset);
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

fn reset_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Idle)
        .apply(|ctx: &mut Context| {
            ctx.valid_drag = false;
            ctx.is_drop_target = false;
            ctx.dropped_items.clear();
            ctx.last_rejection = None;
        })
        .cancel_effect(Effect::ArmDropActivate)
        .cancel_effect(Effect::ResetAfterDrop)
}

fn drop_enter_effect(data: DragData) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::DropEnter, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_drop_enter {
            callback(data);
        }

        no_cleanup()
    })
}

fn drop_move_effect(data: DragData) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::DropMove, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_drop_move {
            callback(data);
        }

        no_cleanup()
    })
}

fn drop_exit_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::DropExit, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_drop_exit {
            callback();
        }

        no_cleanup()
    })
}

fn drop_activate_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::DropActivate, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_drop_activate {
            callback();
        }

        no_cleanup()
    })
}

fn drop_accepted_effect(items: Vec<DragItem>) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::DropAccepted,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_drop {
                callback(items);
            }

            no_cleanup()
        },
    )
}

fn drop_rejected_effect(rejection: DropRejection) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::DropRejected,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_drop_rejected {
                callback(rejection);
            }

            no_cleanup()
        },
    )
}

fn validate_drag_types(ctx: &Context, types: &[String]) -> bool {
    if ctx.accept.is_empty() {
        true
    } else {
        types
            .iter()
            .any(|mime_type| mime_type_accepted(&ctx.accept, mime_type))
    }
}

fn validate_drop(ctx: &Context, items: &[DragItem]) -> Vec<DropValidationError> {
    let mut errors = Vec::new();

    if let Some(max) = ctx.max_files
        && items.len() > max
    {
        errors.push(DropValidationError::TooManyFiles {
            actual: items.len(),
            max,
        });
    }

    if let Some(max) = ctx.max_file_size {
        for item in items {
            if let DragItem::File { name, size, .. } = item
                && *size > max
            {
                errors.push(DropValidationError::FileTooLarge {
                    name: name.clone(),
                    size: *size,
                    max,
                });
            }
        }
    }

    if !ctx.accept.is_empty() {
        for item in items {
            let Some(mime_type) = item_mime_type(item) else {
                continue;
            };

            let normalized = normalize_mime_type(&mime_type);
            if !accepted_mime_matches(&ctx.accept, &normalized) {
                errors.push(DropValidationError::UnsupportedType {
                    mime_type: normalized,
                });
            }
        }
    }

    errors
}

fn item_mime_type(item: &DragItem) -> Option<String> {
    match item {
        DragItem::Text(_) => Some(String::from("text/plain")),

        DragItem::Uri(_) => Some(String::from("text/uri-list")),

        DragItem::Html(_) => Some(String::from("text/html")),

        DragItem::File { mime_type, .. } | DragItem::Custom { mime_type, .. } => {
            Some(mime_type.clone())
        }

        DragItem::Directory { .. } => None,
    }
}

fn mime_type_accepted(accepted_types: &[String], mime_type: &str) -> bool {
    accepted_mime_matches(accepted_types, &normalize_mime_type(mime_type))
}

fn accepted_mime_matches(accepted_types: &[String], actual: &str) -> bool {
    accepted_types
        .iter()
        .map(String::as_str)
        .map(normalize_mime_type)
        .any(|accepted| mime_type_matches(&accepted, actual))
}

fn mime_type_matches(accepted: &str, actual: &str) -> bool {
    if let Some(prefix) = accepted.strip_suffix("/*") {
        actual
            .split_once('/')
            .is_some_and(|(actual_prefix, _)| actual_prefix == prefix)
    } else {
        accepted == actual
    }
}

fn normalize_mime_type(mime_type: &str) -> String {
    let normalized = mime_type.trim().to_ascii_lowercase();

    if normalized == "image/jpg" {
        "image/jpeg".to_owned()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use alloc::{sync::Arc, vec, vec::Vec};
    use std::sync::Mutex;

    use ars_core::{
        AriaAttr, AttrMap, Callback, ConnectApi, Env, HtmlAttr, Service, StrongSend, callback,
    };
    use ars_interactions::{DragItem, FileHandle};
    use insta::assert_snapshot;

    use super::*;

    fn file(name: &str, mime_type: &str, size: u64) -> DragItem {
        DragItem::File {
            name: name.into(),
            mime_type: mime_type.into(),
            size,
            handle: FileHandle::opaque(),
        }
    }

    fn drag_data(items: Vec<DragItem>, types: &[&str]) -> DragData {
        DragData {
            items,
            types: types.iter().map(|item| (*item).into()).collect(),
        }
    }

    fn props() -> Props {
        Props::new().id("uploads").label("Upload files")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn drop_zone_initial_state_is_idle() {
        let service = service(props());

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().valid_drag);
        assert!(!service.context().is_drop_target);
        assert!(service.context().dropped_items.is_empty());
        assert!(service.context().last_rejection.is_none());
    }

    #[test]
    fn drop_zone_drag_enter_transitions_idle_to_drag_over() {
        let mut service = service(props().accept(["image/*"]));

        let result = service.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"])));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::DragOver);
        assert!(service.context().valid_drag);
        assert!(service.context().is_drop_target);
        assert_eq!(result.pending_effects[0].name, Effect::DropEnter);
        assert_eq!(result.pending_effects[1].name, Effect::ArmDropActivate);
    }

    #[test]
    fn drop_zone_drag_leave_transitions_drag_over_to_idle() {
        let mut service = service(props());

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));

        let result = service.send(Event::DragLeave);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().valid_drag);
        assert!(!service.context().is_drop_target);
        assert_eq!(result.pending_effects[0].name, Effect::DropExit);
        assert_eq!(result.cancel_effects, vec![Effect::ArmDropActivate]);
    }

    #[test]
    fn drop_zone_drop_accepts_valid_items_and_exposes_form_data() {
        let mut service = service(
            props()
                .accept(["image/*"])
                .max_files(2)
                .max_file_size(2_000)
                .name("files"),
        );

        let items = vec![file("avatar.jpg", "image/jpg", 1_024)];

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["image/jpeg"]))));

        let result = service.send(Event::Drop(drag_data(items.clone(), &["image/jpeg"])));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::DropAccepted);
        assert_eq!(service.context().dropped_items, items);
        assert_eq!(
            service.connect(&|_| {}).form_data(),
            service.context().dropped_items.as_slice()
        );
        assert_eq!(result.pending_effects[0].name, Effect::DropAccepted);
        assert_eq!(result.pending_effects[1].name, Effect::ResetAfterDrop);
        assert_eq!(result.cancel_effects, vec![Effect::ArmDropActivate]);
    }

    #[test]
    fn drop_zone_drop_rejects_and_aggregates_validation_failures() {
        let mut service = service(
            props()
                .accept(["image/png"])
                .max_files(1)
                .max_file_size(100),
        );

        let items = vec![
            file("huge.txt", "text/plain", 1_000),
            file("icon.jpg", "image/jpeg", 200),
        ];

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["text/plain"]))));

        let result = service.send(Event::Drop(drag_data(items.clone(), &["text/plain"])));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::DropRejected);

        let rejection = service
            .context()
            .last_rejection
            .as_ref()
            .expect("invalid drop records rejection details");

        assert_eq!(rejection.data.items, items);
        assert_eq!(
            rejection.errors,
            vec![
                DropValidationError::TooManyFiles { actual: 2, max: 1 },
                DropValidationError::FileTooLarge {
                    name: "huge.txt".into(),
                    size: 1_000,
                    max: 100,
                },
                DropValidationError::FileTooLarge {
                    name: "icon.jpg".into(),
                    size: 200,
                    max: 100,
                },
                DropValidationError::UnsupportedType {
                    mime_type: "text/plain".into(),
                },
                DropValidationError::UnsupportedType {
                    mime_type: "image/jpeg".into(),
                },
            ],
        );
        assert_eq!(result.pending_effects[0].name, Effect::DropRejected);
        assert_eq!(result.pending_effects[1].name, Effect::ResetAfterDrop);
    }

    #[test]
    fn drop_zone_reset_clears_terminal_states() {
        let mut service = service(props().accept(["image/png"]));

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["text/plain"]))));
        drop(service.send(Event::Drop(drag_data(
            vec![DragItem::Text("bad".into())],
            &["text/plain"],
        ))));

        assert_eq!(service.state(), &State::DropRejected);

        let result = service.send(Event::Reset);

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().dropped_items.is_empty());
        assert!(service.context().last_rejection.is_none());
        assert!(result.state_changed);
    }

    #[test]
    fn drop_zone_effect_callbacks_fire_for_drop_lifecycle() {
        let accepted = Arc::new(Mutex::new(Vec::new()));
        let rejected = Arc::new(Mutex::new(Vec::new()));
        let activated = Arc::new(Mutex::new(0));
        let entered = Arc::new(Mutex::new(Vec::new()));
        let moved = Arc::new(Mutex::new(Vec::new()));
        let exited = Arc::new(Mutex::new(0));

        let mut service = service(
            props()
                .accept(["image/png"])
                .on_drop_enter(callback({
                    let entered = Arc::clone(&entered);
                    move |data: DragData| entered.lock().unwrap().push(data)
                }))
                .on_drop_move(callback({
                    let moved = Arc::clone(&moved);
                    move |data: DragData| moved.lock().unwrap().push(data)
                }))
                .on_drop_exit(Callback::from({
                    let exited = Arc::clone(&exited);
                    move || *exited.lock().unwrap() += 1
                }))
                .on_drop(callback({
                    let accepted = Arc::clone(&accepted);
                    move |items: Vec<DragItem>| accepted.lock().unwrap().extend(items)
                }))
                .on_drop_rejected(callback({
                    let rejected = Arc::clone(&rejected);
                    move |rejection: DropRejection| rejected.lock().unwrap().push(rejection)
                }))
                .on_drop_activate(Callback::from({
                    let activated = Arc::clone(&activated);
                    move || *activated.lock().unwrap() += 1
                })),
        );

        let enter = service.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"])));
        let activate = service.send(Event::DropActivate);

        let send: StrongSend<Event> = Arc::new(|_| {});
        for effect in enter.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        for effect in activate.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        let moved_data = drag_data(Vec::new(), &["IMAGE/JPG"]);

        let drag_move = service.send(Event::DragOver(moved_data.clone()));

        for effect in drag_move.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        let leave = service.send(Event::DragLeave);

        for effect in leave.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));

        let accept = service.send(Event::Drop(drag_data(
            vec![file("icon.png", "image/png", 10)],
            &["image/png"],
        )));

        for effect in accept.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["text/plain"]))));

        let reject = service.send(Event::Drop(drag_data(
            vec![DragItem::Text("bad".into())],
            &["text/plain"],
        )));

        for effect in reject.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(*activated.lock().unwrap(), 1);
        assert_eq!(entered.lock().unwrap().len(), 1);
        assert_eq!(*moved.lock().unwrap(), vec![moved_data]);
        assert_eq!(*exited.lock().unwrap(), 1);
        assert_eq!(
            *accepted.lock().unwrap(),
            vec![file("icon.png", "image/png", 10)]
        );
        assert_eq!(rejected.lock().unwrap().len(), 1);
    }

    #[test]
    fn drop_zone_props_builder_round_trips_all_fields() {
        let props = Props::new()
            .id("uploads")
            .accept([" image/jpg ", "TEXT/*"])
            .max_files(3)
            .clear_max_files()
            .max_files(2)
            .max_file_size(4096)
            .clear_max_file_size()
            .max_file_size(2048)
            .disabled(true)
            .label("Upload files")
            .allowed_operations([DropOperation::Copy, DropOperation::Link])
            .name("files")
            .clear_name()
            .name("uploads[]")
            .required(true)
            .invalid(true)
            .read_only(true)
            .activate_delay(Duration::from_millis(250))
            .reset_delay(Duration::from_millis(750))
            .get_drop_operation(Callback::from(
                |(_data, operations): (DragData, Vec<DropOperation>)| operations[0],
            ))
            .on_drop(callback(|_items: Vec<DragItem>| {}))
            .on_drop_rejected(callback(|_rejection: DropRejection| {}))
            .on_drop_enter(callback(|_data: DragData| {}))
            .on_drop_exit(Callback::from(|| {}))
            .on_drop_move(callback(|_data: DragData| {}))
            .on_hover_start(Callback::from(|| {}))
            .on_drop_activate(Callback::from(|| {}))
            .on_hover_end(Callback::from(|| {}));

        assert_eq!(props.id, "uploads");
        assert_eq!(props.accept, vec![" image/jpg ", "TEXT/*"]);
        assert_eq!(props.max_files, Some(2));
        assert_eq!(props.max_file_size, Some(2048));
        assert!(props.disabled);
        assert_eq!(props.label, "Upload files");
        assert_eq!(
            props.allowed_operations,
            vec![DropOperation::Copy, DropOperation::Link]
        );
        assert_eq!(props.name.as_deref(), Some("uploads[]"));
        assert!(props.required);
        assert!(props.invalid);
        assert!(props.read_only);
        assert_eq!(props.activate_delay, Duration::from_millis(250));
        assert_eq!(props.reset_delay, Duration::from_millis(750));
        assert!(props.get_drop_operation.is_some());
        assert!(props.on_drop.is_some());
        assert!(props.on_drop_rejected.is_some());
        assert!(props.on_drop_enter.is_some());
        assert!(props.on_drop_exit.is_some());
        assert!(props.on_drop_move.is_some());
        assert!(props.on_hover_start.is_some());
        assert!(props.on_drop_activate.is_some());
        assert!(props.on_hover_end.is_some());
    }

    #[test]
    fn drop_zone_disabled_and_read_only_guards_are_precise() {
        let mut disabled = service(props().disabled(true).name("files"));

        assert!(
            !disabled
                .send(Event::DragEnter(drag_data(Vec::new(), &["image/png"])))
                .state_changed
        );
        assert_eq!(disabled.state(), &State::Idle);

        drop(disabled.send(Event::Focus { is_keyboard: true }));

        assert!(disabled.context().focused);
        assert!(disabled.context().focus_visible);

        let reset = disabled.send(Event::Reset);

        assert!(!reset.state_changed);
        assert!(!reset.context_changed);

        let set_props = disabled.set_props(props().disabled(false).accept(["text/plain"]));

        assert!(!disabled.context().disabled);
        assert_eq!(disabled.context().accept, vec!["text/plain"]);
        assert!(set_props.context_changed);

        let mut resettable = service(props());

        drop(resettable.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));
        drop(resettable.send(Event::Drop(drag_data(
            vec![file("icon.png", "image/png", 10)],
            &["image/png"],
        ))));
        drop(resettable.set_props(props().disabled(true)));

        assert_eq!(resettable.state(), &State::DropAccepted);

        let reset = resettable.send(Event::Reset);

        assert!(reset.state_changed);
        assert_eq!(resettable.state(), &State::Idle);

        let mut read_only = service(props().read_only(true));

        let read_only_drag =
            read_only.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"])));

        assert!(!read_only_drag.state_changed);
        assert_eq!(read_only.state(), &State::Idle);

        drop(read_only.send(Event::Focus { is_keyboard: false }));

        assert!(read_only.context().focused);
        assert!(!read_only.context().focus_visible);
    }

    #[test]
    fn drop_zone_drag_over_revalidates_and_blur_clears_focus() {
        let mut service = service(props().accept(["image/png"]));

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));

        assert!(service.context().valid_drag);

        let move_result = service.send(Event::DragOver(drag_data(Vec::new(), &["text/plain"])));

        assert!(!service.context().valid_drag);
        assert_eq!(move_result.pending_effects[0].name, Effect::DropMove);

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn drop_zone_set_props_syncs_every_behavioral_field() {
        let old = props()
            .accept(["image/png"])
            .max_files(1)
            .max_file_size(100)
            .disabled(false)
            .read_only(false);

        let cases = [
            old.clone().accept(["text/plain"]),
            old.clone().max_files(2),
            old.clone().max_file_size(200),
            old.clone().disabled(true),
            old.clone().read_only(true),
        ];

        for new in cases {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                vec![Event::SetProps]
            );
        }

        assert!(
            <Machine as ars_core::Machine>::on_props_changed(
                &old,
                &old.clone().label("Render-only label")
            )
            .is_empty()
        );

        let mut service = service(old);

        drop(
            service.set_props(
                props()
                    .accept(["text/html"])
                    .max_files(3)
                    .max_file_size(300)
                    .disabled(true)
                    .read_only(true),
            ),
        );

        assert_eq!(service.context().accept, vec!["text/html"]);
        assert_eq!(service.context().max_files, Some(3));
        assert_eq!(service.context().max_file_size, Some(300));
        assert!(service.context().disabled);
        assert!(service.context().read_only);
    }

    #[test]
    fn drop_zone_form_data_guards_name_and_disabled_independently() {
        let items = vec![file("icon.png", "image/png", 10)];

        let mut named = service(props().name("files"));

        drop(named.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));
        drop(named.send(Event::Drop(drag_data(items.clone(), &["image/png"]))));

        assert_eq!(named.connect(&|_| {}).form_data(), items.as_slice());

        let unnamed = service(props());

        assert!(unnamed.connect(&|_| {}).form_data().is_empty());

        drop(named.set_props(props().name("files").disabled(true)));

        assert!(named.connect(&|_| {}).form_data().is_empty());
    }

    #[test]
    fn drop_zone_api_event_helpers_dispatch_typed_events() {
        let service = service(props());
        let sent = Arc::new(Mutex::new(Vec::new()));

        let send = {
            let sent = Arc::clone(&sent);
            move |event: Event| sent.lock().unwrap().push(event)
        };

        let api = service.connect(&send);

        let enter = drag_data(Vec::new(), &["image/png"]);
        let over = drag_data(Vec::new(), &["text/plain"]);
        let drop = drag_data(vec![file("icon.png", "image/png", 10)], &["image/png"]);

        api.on_focus(true);
        api.on_blur();
        api.on_drag_enter(enter.clone());
        api.on_drag_over(over.clone());
        api.on_drag_leave();
        api.on_drop(drop.clone());
        api.on_form_reset();

        assert_eq!(
            *sent.lock().unwrap(),
            vec![
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::DragEnter(enter),
                Event::DragOver(over),
                Event::DragLeave,
                Event::Drop(drop),
                Event::Reset,
            ]
        );
    }

    #[test]
    fn drop_zone_validation_boundaries_are_inclusive() {
        let mut service = service(
            props()
                .accept(["image/jpeg"])
                .max_files(2)
                .max_file_size(100),
        );

        let boundary_items = vec![
            file("one.jpg", "image/jpg", 100),
            file("two.jpg", "IMAGE/JPEG", 100),
        ];

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["image/jpg"]))));

        let result = service.send(Event::Drop(drag_data(
            boundary_items.clone(),
            &[" image/jpeg "],
        )));

        assert_eq!(service.state(), &State::DropAccepted);
        assert_eq!(service.context().dropped_items, boundary_items);
        assert_eq!(result.pending_effects[0].name, Effect::DropAccepted);
    }

    #[test]
    fn drop_zone_root_attrs_match_accessibility_contract() {
        let mut service = service(props().required(true));

        let idle = service.connect(&|_| {}).root_attrs();

        assert_eq!(idle.get(&HtmlAttr::Data("ars-scope")), Some("drop-zone"));
        assert_eq!(idle.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(idle.get(&HtmlAttr::Data("ars-state")), Some("idle"));
        assert_eq!(idle.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(idle.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(
            idle.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Upload files")
        );
        assert_eq!(idle.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
        assert_eq!(idle.get(&HtmlAttr::Aria(AriaAttr::DropEffect)), None);
        assert_eq!(idle.get(&HtmlAttr::Data("ars-dragging-over")), None);

        drop(service.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));
        drop(service.send(Event::Focus { is_keyboard: true }));

        let drag_over = service.connect(&|_| {}).root_attrs();

        assert_eq!(
            drag_over.get(&HtmlAttr::Data("ars-state")),
            Some("drag-over")
        );
        assert_eq!(
            drag_over.get(&HtmlAttr::Data("ars-drag-over")),
            Some("true")
        );
        assert_eq!(
            drag_over.get(&HtmlAttr::Aria(AriaAttr::Description)),
            Some("Release to drop files"),
        );
        assert_eq!(
            drag_over.get(&HtmlAttr::Data("ars-focus-visible")),
            Some("true")
        );
    }

    #[test]
    fn drop_zone_connect_api_part_attrs_matches_root() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn drop_zone_api_exposes_localized_drop_result_announcements() {
        let messages = Messages {
            drop_accepted_announcement: MessageFn::new(|locale: &Locale| {
                format!("accepted [{}]", locale.to_bcp47())
            }),
            drop_rejected_announcement: MessageFn::new(|locale: &Locale| {
                format!("rejected [{}]", locale.to_bcp47())
            }),
            ..Messages::default()
        };

        let service = Service::<Machine>::new(props(), &Env::default(), &messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.drop_accepted_announcement(), "accepted [en-US]");
        assert_eq!(api.drop_rejected_announcement(), "rejected [en-US]");
    }

    #[test]
    fn drop_zone_snapshots_cover_output_branches() {
        assert_snapshot!(
            "drop_zone_root_idle_explicit_label",
            snapshot_attrs(&service(props()).connect(&|_| {}).root_attrs())
        );

        assert_snapshot!(
            "drop_zone_root_idle_default_label",
            snapshot_attrs(
                &service(Props::new().id("uploads"))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );

        let mut valid = service(props().accept(["image/*"]));

        drop(valid.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));

        assert_snapshot!(
            "drop_zone_root_drag_over_valid",
            snapshot_attrs(&valid.connect(&|_| {}).root_attrs())
        );

        let mut invalid = service(props().accept(["image/png"]));

        drop(invalid.send(Event::DragEnter(drag_data(Vec::new(), &["text/plain"]))));

        assert_snapshot!(
            "drop_zone_root_drag_over_invalid",
            snapshot_attrs(&invalid.connect(&|_| {}).root_attrs())
        );

        let mut accepted = service(props());

        drop(accepted.send(Event::DragEnter(drag_data(Vec::new(), &["image/png"]))));
        drop(accepted.send(Event::Drop(drag_data(
            vec![file("icon.png", "image/png", 10)],
            &["image/png"],
        ))));

        assert_snapshot!(
            "drop_zone_root_accepted",
            snapshot_attrs(&accepted.connect(&|_| {}).root_attrs())
        );

        let mut rejected = service(props().accept(["image/png"]));

        drop(rejected.send(Event::DragEnter(drag_data(Vec::new(), &["text/plain"]))));
        drop(rejected.send(Event::Drop(drag_data(
            vec![DragItem::Text("bad".into())],
            &["text/plain"],
        ))));

        assert_snapshot!(
            "drop_zone_root_rejected",
            snapshot_attrs(&rejected.connect(&|_| {}).root_attrs())
        );

        assert_snapshot!(
            "drop_zone_root_disabled",
            snapshot_attrs(
                &service(props().disabled(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );

        assert_snapshot!(
            "drop_zone_root_readonly",
            snapshot_attrs(
                &service(props().read_only(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );

        let mut invalid_required = service(props().required(true).invalid(true));

        drop(invalid_required.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "drop_zone_root_required_invalid_focus_visible",
            snapshot_attrs(&invalid_required.connect(&|_| {}).root_attrs())
        );
    }
}
