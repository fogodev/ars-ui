//! Live-region component state machine and connect API.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_a11y::AriaRelevant;
use ars_core::{
    AriaAttr, AttrMap, ComponentPart, ConnectApi, Env, HtmlAttr, PendingEffect, TransitionPlan,
};

/// The state of the `LiveRegion` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No announcement is pending.
    Idle,

    /// A message is moving through the clear-then-insert announcement cycle.
    Announcing,
}

/// The events of the `LiveRegion` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Queue a new announcement.
    Announce {
        /// The message to announce.
        message: String,

        /// The announcement priority.
        priority: AnnouncePriority,
    },

    /// Clear all pending and rendered announcements.
    Clear,

    /// Signal that the adapter has completed one clear-then-insert cycle.
    Rendered,

    /// Synchronize output-affecting props into context.
    SetProps,
}

/// The politeness levels for `aria-live`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaPoliteness {
    /// Do not announce changes.
    Off,

    /// Announce changes politely after the current screen-reader output.
    Polite,

    /// Announce changes assertively, interrupting current output when needed.
    Assertive,
}

/// The priority levels for announcements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnouncePriority {
    /// Use the component's configured live-region politeness.
    Normal,

    /// Force an assertive live-region announcement.
    Urgent,
}

/// Queued announcement data waiting for an active announcement cycle to finish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedAnnouncement {
    /// The message to announce.
    pub message: String,

    /// The announcement priority.
    pub priority: AnnouncePriority,

    /// Monotonic insertion order used to preserve FIFO ordering per priority.
    pub sequence: u64,
}

/// The context of the `LiveRegion` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The messages currently rendered inside the live region.
    pub messages: Vec<String>,

    /// Messages waiting behind the active announcement.
    pub queue: Vec<QueuedAnnouncement>,

    /// The base politeness of the live region.
    pub politeness: AriaPoliteness,

    /// Whether the whole live region is announced for each update.
    pub atomic: bool,

    /// The relevant changes for `aria-relevant`.
    pub relevant: AriaRelevant,

    /// Delay before inserting the pending announcement.
    pub delay: Duration,

    /// Message waiting for the adapter's clear-then-insert cycle.
    pub pending_message: Option<String>,

    /// The priority of the active announcement.
    pub current_priority: AnnouncePriority,
}

/// Props for the `LiveRegion` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// The base politeness of the live region.
    pub politeness: AriaPoliteness,

    /// Whether the whole live region is announced for each update.
    pub atomic: bool,

    /// The relevant changes for `aria-relevant`.
    pub relevant: AriaRelevant,

    /// Delay before inserting the pending announcement.
    pub delay: Duration,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            politeness: AriaPoliteness::Polite,
            atomic: true,
            relevant: AriaRelevant::default(),
            delay: Duration::from_millis(100),
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the stable adapter-provided root ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`politeness`](Self::politeness).
    #[must_use]
    pub const fn politeness(mut self, politeness: AriaPoliteness) -> Self {
        self.politeness = politeness;
        self
    }

    /// Sets [`atomic`](Self::atomic).
    #[must_use]
    pub const fn atomic(mut self, atomic: bool) -> Self {
        self.atomic = atomic;
        self
    }

    /// Sets [`relevant`](Self::relevant).
    #[must_use]
    pub const fn relevant(mut self, relevant: AriaRelevant) -> Self {
        self.relevant = relevant;
        self
    }

    /// Sets [`delay`](Self::delay).
    #[must_use]
    pub const fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ars_core::ComponentMessages for Messages {}

/// Typed effect intents emitted by the live-region machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter performs the delayed clear-then-insert render cycle.
    AnnounceDelay,
}

/// The machine for the `LiveRegion` component.
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
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Idle,
            Context {
                messages: Vec::new(),
                queue: Vec::new(),
                politeness: props.politeness,
                atomic: props.atomic,
                relevant: props.relevant.clone(),
                delay: props.delay,
                pending_message: None,
                current_priority: AnnouncePriority::Normal,
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Idle, Event::Announce { message, priority }) => {
                let message = message.clone();
                let priority = *priority;

                Some(
                    TransitionPlan::to(State::Announcing)
                        .apply(move |ctx: &mut Context| {
                            ctx.messages.clear();
                            ctx.pending_message = Some(message);
                            ctx.current_priority = priority;
                        })
                        .with_effect(announce_delay_effect()),
                )
            }

            (State::Announcing, Event::Announce { message, priority }) => {
                let message = message.clone();
                let priority = *priority;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let sequence = next_sequence(&ctx.queue);
                    ctx.queue.push(QueuedAnnouncement {
                        message,
                        priority,
                        sequence,
                    });
                }))
            }

            (State::Announcing, Event::Rendered) => Some(rendered_plan(!ctx.queue.is_empty())),

            (_, Event::SetProps) => {
                let politeness = props.politeness;
                let atomic = props.atomic;
                let relevant = props.relevant.clone();
                let delay = props.delay;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.politeness = politeness;
                    ctx.atomic = atomic;
                    ctx.relevant = relevant;
                    ctx.delay = delay;
                }))
            }

            (_, Event::Clear) if has_announcements(ctx) => Some(clear_plan(State::Idle)),

            (State::Announcing, Event::Clear) => Some(clear_plan(State::Idle)),

            (State::Idle, Event::Clear | Event::Rendered) => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "live_region::Props.id must remain stable after init"
        );

        if old.politeness != new.politeness
            || old.atomic != new.atomic
            || old.relevant != new.relevant
            || old.delay != new.delay
        {
            vec![Event::SetProps]
        } else {
            Vec::new()
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
}

/// DOM parts of the `LiveRegion` component.
#[derive(ComponentPart)]
#[scope = "live-region"]
pub enum Part {
    /// The root live-region container.
    Root,
}

/// The `LiveRegion` component marker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiveRegion;

/// The API for the `LiveRegion` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("live_region::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// DOM props for the live-region container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Live), self.live_value())
            .set(
                HtmlAttr::Aria(AriaAttr::Atomic),
                if self.props.atomic { "true" } else { "false" },
            )
            .set(HtmlAttr::Data("ars-state"), self.data_state())
            .set(HtmlAttr::Class, "ars-visually-hidden");

        let relevant = self.props.relevant.to_string();
        if !relevant.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Relevant), relevant);
        }

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }

        attrs
    }

    /// Imperatively request an announcement.
    pub fn announce(&self, message: &str, priority: AnnouncePriority) {
        (self.send)(Event::Announce {
            message: message.to_owned(),
            priority,
        });
    }

    /// Clear all pending and rendered announcements.
    pub fn clear(&self) {
        (self.send)(Event::Clear);
    }

    /// Messages currently rendered inside the live-region element.
    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.ctx.messages
    }

    const fn data_state(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Announcing => "announcing",
        }
    }

    const fn live_value(&self) -> &'static str {
        if matches!(self.state, State::Announcing)
            && matches!(self.ctx.current_priority, AnnouncePriority::Urgent)
        {
            return "assertive";
        }

        politeness_value(self.props.politeness)
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

fn rendered_plan(has_next: bool) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(if has_next {
        State::Announcing
    } else {
        State::Idle
    })
    .apply(|ctx: &mut Context| {
        if let Some(message) = ctx.pending_message.take() {
            ctx.messages.clear();

            ctx.messages.push(message);
        }
    })
    .apply(|ctx: &mut Context| {
        if let Some(next) = dequeue_next(&mut ctx.queue) {
            ctx.pending_message = Some(next.message);
            ctx.current_priority = next.priority;
        } else {
            ctx.current_priority = AnnouncePriority::Normal;
        }
    });

    if has_next {
        plan = plan.with_effect(announce_delay_effect());
    }

    plan
}

fn clear_plan(target: State) -> TransitionPlan<Machine> {
    TransitionPlan::to(target)
        .apply(|ctx: &mut Context| {
            ctx.messages.clear();
            ctx.queue.clear();
            ctx.pending_message = None;
            ctx.current_priority = AnnouncePriority::Normal;
        })
        .cancel_effect(Effect::AnnounceDelay)
}

const fn has_announcements(ctx: &Context) -> bool {
    !ctx.messages.is_empty() || !ctx.queue.is_empty() || ctx.pending_message.is_some()
}

fn announce_delay_effect() -> PendingEffect<Machine> {
    PendingEffect::named(Effect::AnnounceDelay)
}

fn next_sequence(queue: &[QueuedAnnouncement]) -> u64 {
    queue
        .iter()
        .map(|announcement| announcement.sequence)
        .max()
        .map_or(0, |sequence| sequence.saturating_add(1))
}

fn dequeue_next(queue: &mut Vec<QueuedAnnouncement>) -> Option<QueuedAnnouncement> {
    let index = queue
        .iter()
        .enumerate()
        .min_by_key(|(_, announcement)| {
            (
                priority_sort_key(announcement.priority),
                announcement.sequence,
            )
        })
        .map(|(index, _)| index)?;

    Some(queue.remove(index))
}

const fn priority_sort_key(priority: AnnouncePriority) -> u8 {
    match priority {
        AnnouncePriority::Urgent => 0,
        AnnouncePriority::Normal => 1,
    }
}

const fn politeness_value(politeness: AriaPoliteness) -> &'static str {
    match politeness {
        AriaPoliteness::Off => "off",
        AriaPoliteness::Polite => "polite",
        AriaPoliteness::Assertive => "assertive",
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, sync::Arc, vec, vec::Vec};
    use core::time::Duration;
    use std::sync::Mutex;

    use ars_a11y::AriaRelevant;
    use ars_core::{AriaAttr, AttrMap, ConnectApi as _, Env, HtmlAttr, Service};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "announcer".into(),
            ..Props::default()
        }
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages)
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn live_region_initial_state_is_idle() {
        let service = service(test_props());

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().messages.is_empty());
        assert!(service.context().queue.is_empty());
        assert_eq!(service.context().pending_message, None);
        assert_eq!(service.context().politeness, AriaPoliteness::Polite);
        assert!(service.context().atomic);
        assert_eq!(service.context().relevant, AriaRelevant::default());
        assert_eq!(service.context().delay, Duration::from_millis(100));
        assert_eq!(service.context().current_priority, AnnouncePriority::Normal);
    }

    #[test]
    fn live_region_props_builder_sets_expected_fields() {
        let relevant = AriaRelevant {
            additions: true,
            removals: true,
            text: false,
        };

        let props = Props::new()
            .id("status")
            .politeness(AriaPoliteness::Assertive)
            .atomic(false)
            .relevant(relevant.clone())
            .delay(Duration::from_millis(250));

        assert_eq!(props.id, "status");
        assert_eq!(props.politeness, AriaPoliteness::Assertive);
        assert!(!props.atomic);
        assert_eq!(props.relevant, relevant);
        assert_eq!(props.delay, Duration::from_millis(250));
    }

    #[test]
    fn announce_from_idle_transitions_to_announcing_and_emits_delay_effect() {
        let mut service = service(test_props());

        let result = service.send(Event::Announce {
            message: "Saved".into(),
            priority: AnnouncePriority::Normal,
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Announcing);
        assert!(service.context().messages.is_empty());
        assert_eq!(service.context().pending_message.as_deref(), Some("Saved"));
        assert_eq!(service.context().current_priority, AnnouncePriority::Normal);
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::AnnounceDelay);
    }

    #[test]
    fn rendered_moves_pending_message_into_messages() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Saved".into(),
            priority: AnnouncePriority::Normal,
        }));

        let result = service.send(Event::Rendered);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().messages, vec![String::from("Saved")]);
        assert_eq!(service.context().pending_message, None);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn rendered_dequeues_next_message_when_queue_not_empty() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "First".into(),
            priority: AnnouncePriority::Normal,
        }));
        drop(service.send(Event::Announce {
            message: "Second".into(),
            priority: AnnouncePriority::Normal,
        }));

        let result = service.send(Event::Rendered);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Announcing);
        assert_eq!(service.context().messages, vec![String::from("First")]);
        assert_eq!(service.context().pending_message.as_deref(), Some("Second"));
        assert!(service.context().queue.is_empty());
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::AnnounceDelay);
    }

    #[test]
    fn urgent_messages_dequeue_before_normal_messages() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Active".into(),
            priority: AnnouncePriority::Normal,
        }));
        drop(service.send(Event::Announce {
            message: "Queued normal".into(),
            priority: AnnouncePriority::Normal,
        }));
        drop(service.send(Event::Announce {
            message: "Queued urgent".into(),
            priority: AnnouncePriority::Urgent,
        }));

        drop(service.send(Event::Rendered));

        assert_eq!(
            service.context().pending_message.as_deref(),
            Some("Queued urgent")
        );
        assert_eq!(service.context().current_priority, AnnouncePriority::Urgent);
        assert_eq!(service.context().queue[0].message, "Queued normal");
    }

    #[test]
    fn same_priority_messages_dequeue_fifo() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Active".into(),
            priority: AnnouncePriority::Normal,
        }));
        drop(service.send(Event::Announce {
            message: "First queued".into(),
            priority: AnnouncePriority::Urgent,
        }));
        drop(service.send(Event::Announce {
            message: "Second queued".into(),
            priority: AnnouncePriority::Urgent,
        }));

        drop(service.send(Event::Rendered));

        assert_eq!(
            service.context().pending_message.as_deref(),
            Some("First queued")
        );

        drop(service.send(Event::Rendered));

        assert_eq!(
            service.context().pending_message.as_deref(),
            Some("Second queued")
        );
    }

    #[test]
    fn queued_announcements_get_monotonic_sequence_numbers() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Active".into(),
            priority: AnnouncePriority::Normal,
        }));

        for message in ["First queued", "Second queued", "Third queued"] {
            drop(service.send(Event::Announce {
                message: message.into(),
                priority: AnnouncePriority::Normal,
            }));
        }

        assert_eq!(
            service
                .context()
                .queue
                .iter()
                .map(|announcement| announcement.sequence)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn clear_from_announcing_returns_idle_and_cancels_delay() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Active".into(),
            priority: AnnouncePriority::Urgent,
        }));
        drop(service.send(Event::Announce {
            message: "Queued".into(),
            priority: AnnouncePriority::Normal,
        }));

        let result = service.send(Event::Clear);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().messages.is_empty());
        assert!(service.context().queue.is_empty());
        assert_eq!(service.context().pending_message, None);
        assert_eq!(service.context().current_priority, AnnouncePriority::Normal);
        assert_eq!(result.cancel_effects, vec![Effect::AnnounceDelay]);
    }

    #[test]
    fn clear_from_announcing_without_content_still_cancels_delay() {
        let props = test_props();

        let (_state, ctx) =
            <Machine as ars_core::Machine>::init(&props, &Env::default(), &Messages);

        let result = <Machine as ars_core::Machine>::transition(
            &State::Announcing,
            &Event::Clear,
            &ctx,
            &props,
        )
        .expect("announcing clear should cancel in-flight delay work");

        assert_eq!(result.target, Some(State::Idle));
        assert_eq!(result.cancel_effects, vec![Effect::AnnounceDelay]);
    }

    #[test]
    fn clear_from_idle_clears_rendered_messages() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Rendered".into(),
            priority: AnnouncePriority::Normal,
        }));
        drop(service.send(Event::Rendered));

        let result = service.send(Event::Clear);

        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().messages.is_empty());
        assert_eq!(result.cancel_effects, vec![Effect::AnnounceDelay]);
    }

    #[test]
    fn clear_from_idle_clears_any_announcement_slot() {
        let props = test_props();

        let (_state, base_ctx) =
            <Machine as ars_core::Machine>::init(&props, &Env::default(), &Messages);

        let queue_only = Context {
            queue: vec![QueuedAnnouncement {
                message: "Queued".into(),
                priority: AnnouncePriority::Normal,
                sequence: 0,
            }],
            ..base_ctx.clone()
        };

        let pending_only = Context {
            pending_message: Some("Pending".into()),
            ..base_ctx
        };

        for ctx in [&queue_only, &pending_only] {
            let plan = <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::Clear,
                ctx,
                &props,
            )
            .expect("idle clear should clear stale announcement slots");

            assert_eq!(plan.target, Some(State::Idle));
            assert_eq!(plan.cancel_effects, vec![Effect::AnnounceDelay]);
        }
    }

    #[test]
    fn idle_clear_and_idle_rendered_are_noops() {
        let mut service = service(test_props());

        let clear = service.send(Event::Clear);
        let rendered = service.send(Event::Rendered);

        assert!(!clear.state_changed);
        assert!(!clear.context_changed);
        assert!(clear.pending_effects.is_empty());
        assert!(!rendered.state_changed);
        assert!(!rendered.context_changed);
        assert!(rendered.pending_effects.is_empty());
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn set_props_syncs_output_affecting_context() {
        let mut service = service(test_props());

        let relevant = AriaRelevant {
            additions: false,
            removals: true,
            text: true,
        };

        let result = service.set_props(
            Props::new()
                .id("announcer")
                .politeness(AriaPoliteness::Assertive)
                .atomic(false)
                .relevant(relevant.clone())
                .delay(Duration::from_millis(300)),
        );

        assert!(result.context_changed);
        assert_eq!(service.context().politeness, AriaPoliteness::Assertive);
        assert!(!service.context().atomic);
        assert_eq!(service.context().relevant, relevant);
        assert_eq!(service.context().delay, Duration::from_millis(300));
    }

    #[test]
    fn on_props_changed_emits_set_props_for_politeness_atomic_relevant_delay() {
        let old = test_props();

        for new in [
            Props::new().id("announcer").politeness(AriaPoliteness::Off),
            Props::new().id("announcer").atomic(false),
            Props::new().id("announcer").relevant(AriaRelevant {
                additions: false,
                removals: true,
                text: false,
            }),
            Props::new()
                .id("announcer")
                .delay(Duration::from_millis(250)),
        ] {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                vec![Event::SetProps]
            );
        }
    }

    #[test]
    fn on_props_changed_returns_no_events_for_equal_output_props() {
        let props = test_props();

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&props, &props),
            Vec::<Event>::new()
        );
    }

    #[test]
    #[should_panic(expected = "live_region::Props.id must remain stable after init")]
    fn on_props_changed_panics_when_id_changes() {
        let old = Props::new().id("before");
        let new = Props::new().id("after");

        drop(<Machine as ars_core::Machine>::on_props_changed(&old, &new));
    }

    #[test]
    fn api_announce_dispatches_announce_event() {
        let service = service(test_props());
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let send = move |event| captured.lock().unwrap().push(event);

        let api = service.connect(&send);

        api.announce("Saved", AnnouncePriority::Urgent);

        assert_eq!(
            *events.lock().unwrap(),
            vec![Event::Announce {
                message: "Saved".into(),
                priority: AnnouncePriority::Urgent,
            }]
        );
    }

    #[test]
    fn api_clear_dispatches_clear_event() {
        let service = service(test_props());
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let send = move |event| captured.lock().unwrap().push(event);

        let api = service.connect(&send);

        api.clear();

        assert_eq!(*events.lock().unwrap(), vec![Event::Clear]);
    }

    #[test]
    fn api_debug_includes_state_context_and_props() {
        let service = service(test_props());
        let debug = format!("{:?}", service.connect(&|_| {}));

        assert!(debug.contains("live_region::Api"));
        assert!(debug.contains("state"));
        assert!(debug.contains("ctx"));
        assert!(debug.contains("props"));
    }

    #[test]
    fn api_messages_exposes_rendered_messages() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Rendered".into(),
            priority: AnnouncePriority::Normal,
        }));
        drop(service.send(Event::Rendered));

        assert_eq!(
            service.connect(&|_| {}).messages(),
            &[String::from("Rendered")]
        );
    }

    #[test]
    fn part_attrs_dispatches_root_attrs() {
        let service = service(test_props());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn root_attrs_do_not_set_role() {
        let service = service(test_props());

        let attrs = service.connect(&|_| {}).root_attrs();

        assert!(!attrs.contains(&HtmlAttr::Role));
    }

    #[test]
    fn root_attrs_match_accessibility_contract() {
        let service = service(test_props());

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("announcer"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Relevant)),
            Some("additions text")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));
        assert_eq!(attrs.get(&HtmlAttr::Class), Some("ars-visually-hidden"));
    }

    #[test]
    fn root_attrs_omit_empty_aria_relevant() {
        let service = service(Props::new().id("announcer").relevant(AriaRelevant {
            additions: false,
            removals: false,
            text: false,
        }));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Relevant)));
    }

    #[test]
    fn root_attrs_honor_assertive_politeness_when_idle() {
        let service = service(
            Props::new()
                .id("announcer")
                .politeness(AriaPoliteness::Assertive),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Live)),
            Some("assertive")
        );
    }

    #[test]
    fn live_region_root_idle_polite() {
        assert_snapshot!(snapshot_attrs(
            &service(test_props()).connect(&|_| {}).root_attrs()
        ));
    }

    #[test]
    fn live_region_root_announcing_normal() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Loading".into(),
            priority: AnnouncePriority::Normal,
        }));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn live_region_root_announcing_urgent() {
        let mut service = service(test_props());

        drop(service.send(Event::Announce {
            message: "Failed".into(),
            priority: AnnouncePriority::Urgent,
        }));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn live_region_root_off_non_atomic_relevant_all() {
        let service = service(
            Props::new()
                .id("announcer")
                .politeness(AriaPoliteness::Off)
                .atomic(false)
                .relevant(AriaRelevant {
                    additions: true,
                    removals: true,
                    text: true,
                }),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }
}
