---
component: LiveRegion
category: utility
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    react-aria: LiveAnnouncer
---

# LiveRegion

An `aria-live` announcement utility for broadcasting dynamic content changes to screen readers
without moving focus. Used to announce search result counts, upload progress, toast notifications,
and any other out-of-band changes.

## 1. State Machine

The timing challenge: screen readers often miss `aria-live` updates if the content is inserted
immediately after mounting. `LiveRegion` uses a two-step update pattern — clear then insert — to
guarantee detection by all major screen readers.

### 1.1 States

| State        | Description                                                                                                |
| ------------ | ---------------------------------------------------------------------------------------------------------- |
| `Idle`       | No announcement pending.                                                                                   |
| `Announcing` | An announcement cycle is active; the DOM may be in the cleared delay phase or contain the current message. |

### 1.2 Events

| Event      | Payload                                       | Description                                                         |
| ---------- | --------------------------------------------- | ------------------------------------------------------------------- |
| `Announce` | `message: String, priority: AnnouncePriority` | Queue a new announcement.                                           |
| `Clear`    | —                                             | Clear rendered, pending, and queued announcements.                  |
| `Rendered` | —                                             | Signal that the adapter has inserted the pending announcement text. |
| `SetProps` | —                                             | Sync props into context (triggered by adapter on prop change).      |

### 1.3 Context

```rust
use core::time::Duration;

/// The state of the `LiveRegion` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No announcement pending.
    Idle,
    /// An announcement cycle is active.
    Announcing,
}

/// The events of the `LiveRegion` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Queue a new announcement.
    Announce {
        /// The message to announce.
        message: String,
        /// The priority of the announcement.
        priority: AnnouncePriority,
    },
    /// Clear rendered, pending, and queued announcements.
    Clear,
    /// Signal that the adapter has inserted the pending announcement text.
    Rendered,
    /// Sync props into context (triggered by adapter on prop change).
    SetProps,
}

/// The politeness levels for `aria-live`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AriaPoliteness {
    /// No announcements are made.
    Off,
    /// Announcements are made in a polite manner.
    Polite,
    /// Announcements are made in an assertive manner.
    Assertive,
}

/// The priority levels for announcements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnouncePriority {
    /// Uses the component's configured politeness (default: Polite).
    Normal,
    /// Forces Assertive — interrupts current screen reader speech.
    Urgent,
}

// `AriaRelevant` — defined in `03-accessibility.md` as the shared
// additions/removals/text struct.

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
    /// The politeness of the announcements.
    pub politeness: AriaPoliteness,
    /// true = the entire live region is announced, not just changed nodes.
    pub atomic: bool,
    /// The relevant changes for `aria-relevant`.
    pub relevant: AriaRelevant,
    /// Delay before announcing. Allows batching rapid updates.
    pub delay: Duration,
    /// Tracks whether we are in the "cleared" phase of the two-step update.
    pub pending_message: Option<String>,
    /// The priority of the current announcement. Used to set `aria-live` dynamically.
    pub current_priority: AnnouncePriority,
}
```

### 1.4 Props

```rust
use core::time::Duration;

/// Props for the `LiveRegion` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The politeness of the announcements.
    pub politeness: AriaPoliteness,
    /// Whether the live region should be announced as atomic.
    pub atomic: bool,
    /// The relevant changes for `aria-relevant`.
    pub relevant: AriaRelevant,
    /// Delay before announcing. Helps avoid announcing transient states.
    pub delay: Duration,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            politeness: AriaPoliteness::Polite,
            atomic: true,
            relevant: AriaRelevant::default(), // additions text
            delay: Duration::from_millis(100),
        }
    }
}
```

### 1.5 Transitions

```text
Idle + Announce(msg, priority)
  → Announcing
  action: clear messages[], set pending_message = msg
  effect: Effect::AnnounceDelay — clear DOM content, wait ctx.delay (default 100ms),
          then insert ctx.pending_message text and send Rendered event

Announcing + Rendered
  → Idle if queue is empty; otherwise Announcing
  action: move pending_message into messages[], clear pending_message;
          if queue has messages, dequeue the next highest-priority item,
          keep the rendered message available until the adapter performs
          the next clear-then-insert cycle, set pending_message, set current_priority,
          and emit Effect::AnnounceDelay

Announcing + Announce(msg, priority)
  → Announcing
  action: enqueue msg with priority and insertion sequence

Announcing + Clear
  → Idle
  action: clear messages[], queue[], pending_message
  effect: cancel Effect::AnnounceDelay

Idle + Clear
  → Idle
  action: clear rendered messages when present; otherwise no-op

Idle + Rendered → Idle (ignored, return None)
```

### 1.6 Full Machine Implementation

```rust
use core::time::Duration;

use ars_core::{AttrMap, ConnectApi, PendingEffect, TransitionPlan};

/// Typed effect intents emitted by the live-region machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter performs the delayed clear-then-insert render cycle.
    AnnounceDelay,
}

/// The machine for the `LiveRegion` component.
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
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
            // ── Announce ────────────────────────────────────────────────
            (State::Idle, Event::Announce { message, priority }) => {
                let msg = message.clone();
                let prio = *priority;
                Some(TransitionPlan::to(State::Announcing)
                    .apply(move |ctx| {
                        ctx.messages.clear();
                        ctx.pending_message = Some(msg);
                        ctx.current_priority = prio;
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceDelay)))
            }
            (State::Announcing, Event::Announce { message, priority }) => {
                let msg = message.clone();
                let prio = *priority;
                Some(TransitionPlan::context_only(move |ctx| {
                    let sequence = next_sequence(&ctx.queue);
                    ctx.queue.push(QueuedAnnouncement {
                        message: msg,
                        priority: prio,
                        sequence,
                    });
                }))
            }

            // ── Rendered ────────────────────────────────────────────────
            (State::Announcing, Event::Rendered) => {
                let has_next = !ctx.queue.is_empty();
                let mut plan = TransitionPlan::to(if has_next {
                    State::Announcing
                } else {
                    State::Idle
                })
                .apply(|ctx| {
                    if let Some(msg) = ctx.pending_message.take() {
                        ctx.messages.clear();
                        ctx.messages.push(msg);
                    }
                    if let Some(next) = dequeue_next(&mut ctx.queue) {
                        ctx.pending_message = Some(next.message);
                        ctx.current_priority = next.priority;
                    } else {
                        ctx.current_priority = AnnouncePriority::Normal;
                    }
                });
                if has_next {
                    plan = plan.with_effect(PendingEffect::named(Effect::AnnounceDelay));
                }
                Some(plan)
            }
            // Rendered while Idle is a no-op (e.g., late timer firing after Clear).
            (State::Idle, Event::Rendered) => None,

            // ── Clear ───────────────────────────────────────────────────
            // **Effect cleanup:** When transitioning from Announcing to Idle via Clear,
            // the adapter MUST cancel the pending `Effect::AnnounceDelay` timer/work.
            (_, Event::Clear) if has_announcements(ctx) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.messages.clear();
                    ctx.queue.clear();
                    ctx.pending_message = None;
                    ctx.current_priority = AnnouncePriority::Normal;
                }).cancel_effect(Effect::AnnounceDelay))
            }
            // Clear while Idle with no content is a no-op.
            (State::Idle, Event::Clear) => None,

            // ── SetProps ────────────────────────────────────────────────
            (_, Event::SetProps) => {
                let politeness = props.politeness;
                let atomic = props.atomic;
                let relevant = props.relevant.clone();
                let delay = props.delay;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.politeness = politeness;
                    ctx.atomic = atomic;
                    ctx.relevant = relevant;
                    ctx.delay = delay;
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        if old.politeness != new.politeness || old.atomic != new.atomic
           || old.relevant != new.relevant || old.delay != new.delay {
            events.push(Event::SetProps);
        }
        events
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

> **Effect `Effect::AnnounceDelay`:** The agnostic machine emits a typed, adapter-resolvable
> intent for the clear-then-insert cycle. The adapter clears the root's rendered text, waits
> `ctx.delay` (default 100ms), inserts `ctx.pending_message`, and sends `Rendered`. A new
> `Announce` received while already `Announcing` is queued; it does not replace the active
> pending message or restart the active timer.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "live-region"]
pub enum Part {
    Root,
}

/// The `LiveRegion` component.
pub struct LiveRegion;

/// The API for the `LiveRegion` component.
pub struct Api<'a> {
    /// The state of the `LiveRegion` component.
    state: &'a State,
    /// The context of the `LiveRegion` component.
    ctx: &'a Context,
    /// The props of the `LiveRegion` component.
    props: &'a Props,
    /// The send callback for the `LiveRegion` component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// DOM props for the live region container.
    /// The container is visually hidden but present in the accessibility tree.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        // aria-live is determined by current_priority when announcing, falling back
        // to the base politeness from props when idle.
        let live_value = if *self.state == State::Announcing {
            match self.ctx.current_priority {
                AnnouncePriority::Urgent => "assertive",
                AnnouncePriority::Normal => match self.props.politeness {
                    AriaPoliteness::Off => "off",
                    AriaPoliteness::Polite => "polite",
                    AriaPoliteness::Assertive => "assertive",
                },
            }
        } else {
            match self.props.politeness {
                AriaPoliteness::Off => "off",
                AriaPoliteness::Polite => "polite",
                AriaPoliteness::Assertive => "assertive",
            }
        };
        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), live_value);
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), if self.props.atomic { "true" } else { "false" });
        attrs.set(
            HtmlAttr::Aria(AriaAttr::Relevant),
            self.props.relevant.to_string(),
        );
        attrs.set(
            HtmlAttr::Data("ars-state"),
            match self.state {
                State::Idle => "idle",
                State::Announcing => "announcing",
            },
        );
        // Static styles via companion stylesheet class (CSP-safe in all strategies).
        attrs.set(HtmlAttr::Class, "ars-visually-hidden");
        attrs
    }

    /// Imperatively announce a message.
    pub fn announce(&self, message: &str, priority: AnnouncePriority) {
        (self.send)(Event::Announce {
            message: message.to_owned(),
            priority,
        });
    }

    /// Clear all pending and active announcements.
    pub fn clear(&self) {
        (self.send)(Event::Clear);
    }

    /// The current list of messages rendered inside the live region.
    /// The framework adapter renders these as text children of the region element.
    pub fn messages(&self) -> &[String] {
        &self.ctx.messages
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
```

**Implementation note — two-step update:**

To guarantee screen readers detect the change, the component:

1. Clears the live region content (renders empty string as the message list).
2. Waits a **minimum 100ms delay** before inserting the new announcement text, then
   fires `Rendered`, which inserts the new message.

> **Cross-reader compatibility:** NVDA requires the clear-then-set pattern with a delay;
> JAWS works with direct replacement. The 100ms gap between clearing and inserting
> announcement text ensures cross-reader compatibility. Using `requestAnimationFrame`
> alone is insufficient — a `setTimeout(100)` (or equivalent) is the minimum reliable
> delay across screen readers.

For `AnnouncePriority::Urgent`, the root attributes report `aria-live="assertive"` while the
urgent message is active. The agnostic core does not create or mutate a second DOM node; adapter
tasks may add framework-specific multi-region rendering later if their specs require it.

## 2. Anatomy

```text
LiveRegion
└── Root    <div>    data-ars-scope="live-region" data-ars-part="root"
                     id="..."
                     aria-live="polite|assertive|off"
                     aria-atomic="true|false"
                     aria-relevant="..."
                     data-ars-state="idle|announcing"
                     (visually hidden)
```

| Part | Element | Key Attributes                                                                                                                                      |
| ---- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root | `<div>` | `id`, `data-ars-scope="live-region"`, `data-ars-part="root"`, `data-ars-state`, `aria-live`, `aria-atomic`, `aria-relevant`, `.ars-visually-hidden` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA role is explicitly set on the root element — the `aria-live` attribute alone is sufficient for screen reader monitoring. The default LiveRegion implementation does not set `role="region"`.
- `aria-live` is set to `"polite"`, `"assertive"`, or `"off"` based on the configured politeness and current announcement priority.
- `aria-atomic="true"` (default) announces the entire region content, even if only part changed. Set to `false` only when streaming incremental updates where each addition should be announced independently.
- `aria-relevant` is set per the configured `AriaRelevant` value.
- **`aria-label` on live regions is for region identification, not announcement content.**
  If a consumer wraps the live region in a named container (e.g., `aria-label="Upload status"`),
  VoiceOver may announce "status region: Upload status" when the region first receives content.
  This label identifies the region — it is not the announcement itself. Announcement content
  comes solely from elements inserted into the live region via the two-step update pattern.
  Consumers should keep region labels short and descriptive, and be aware that some screen readers
  (notably VoiceOver) may prepend the region label to the first announcement.
- **Region naming requirement:** If `role="region"` is applied to the live region container, it
  **must** have an accessible name via `aria-labelledby` or `aria-label`. Without a name,
  `role="region"` is treated as a generic landmark and may confuse screen readers.

### 3.2 Screen Reader Announcements

- Live regions should be mounted on application load (or component mount), not inserted
  dynamically. Inserting a live region and immediately populating it may be missed by some
  screen readers.
- Use `Polite` for informational updates (counts, progress). Use `Assertive` only for errors or
  urgent status that should interrupt reading.

## 4. Internationalization

Message text is consumer-provided. `data-ars-*` attribute values and ARIA attribute names are stable API tokens, not localized. The `aria-live`, `aria-atomic`, and `aria-relevant` values are ARIA specification keywords and are not subject to translation. RTL: no special handling needed.

## 5. Internal Usage in ars-ui

| Component    | `LiveRegion` Usage                                      |
| ------------ | ------------------------------------------------------- |
| `FileUpload` | Announces upload progress ("3 of 5 files uploaded")     |
| `Toast`      | Announces notification text when a toast appears        |
| `Combobox`   | Announces filtered result count ("5 results available") |
| `DatePicker` | Announces selected date after calendar navigation       |
| `Pagination` | Announces page change ("Page 3 of 10")                  |

## 6. Stacked `Toast` Announcements

When used by [Toast](../overlay/toast.md):

1. Toasts are announced in FIFO order (oldest first) via a single shared `aria-live='polite'` region.
2. New toasts queue their announcement text; the live region updates sequentially with a 500ms gap between announcements.
3. Priority toasts (`type='error'`) use `aria-live='assertive'` and bypass the queue.
4. If more than 3 toasts are pending announcement, intermediate toasts are consolidated into a count ("3 more notifications").

## 7. Sound Notifications for `Toast`

When `Toast` uses `LiveRegion` for announcements, optional audio cues follow these rules:

1. Audio playback requires a user gesture to create/resume the `AudioContext` (browser autoplay policy). The adapter creates the `AudioContext` on first user interaction with the page and reuses it.
2. If `AudioContext` is not available (no prior user gesture), sound notifications are silently skipped — no error thrown.
3. Toast accepts an optional `sound: Option<SoundEffect>` prop.
4. Sound is supplementary and must never be the only notification mechanism (visual + `aria-live` are primary).

## 8. SSR Requirement

During SSR, the `<div>` MUST render empty with the correct ARIA attributes. Adapters MUST suppress `Announce` events until hydration is complete to prevent hydration mismatches between the empty server HTML and client-side announced text.

The `aria-live` container element **must** be present in the server-rendered HTML. Screen readers
register live regions at page load time; if the container is inserted later by client-side
JavaScript, assistive technology may not monitor it for changes. For SSR frameworks (Leptos SSR,
Dioxus fullstack), the `LiveRegion`'s root `<div>` with its `aria-live`, `aria-atomic`, and
`aria-relevant` attributes must be emitted during the server render pass. The element's content
can be empty initially — the two-step update pattern will populate it on the client. Hydration
must preserve the existing element rather than replacing it.

## 9. Platform Notes

**Dioxus timer cancellation safety:** When the `Effect::AnnounceDelay` timer fires, its callback calls `send(Event::Rendered)`. If the component unmounts before the timer fires, `use_drop` runs cleanup but the timer callback may still be pending. The timer callback MUST check a cancellation flag (`SharedFlag`) before calling `send()`:

```rust,no_check
let platform = use_platform_effects();
let cancelled = SharedFlag::new(false);
let cancelled_clone = cancelled.clone();
let handle = platform.set_timeout(ctx.delay, Box::new(move || {
    if !cancelled_clone.get() {
        send(Event::Rendered);
    }
}));
// In cleanup:
cancelled.set(true);
platform.clear_timeout(handle);
```

This pattern prevents runtime panics when `send()` writes to a dropped Dioxus signal.

> **Dioxus timing:** The two-step clear-then-insert pattern requires a timed delay
> (minimum 100ms). In Leptos, use `platform.set_timeout()` via `PlatformEffects`. In Dioxus, use
> `spawn(async move { tokio::time::sleep(delay).await; if !cancelled.get() { send(Event::Rendered); } })`
> which works on both Web and Desktop targets via the Dioxus async runtime.
>
> **Dioxus SSR:** The `<div aria-live>` container must appear in server-rendered HTML
> for both Leptos and Dioxus fullstack. In Dioxus fullstack with `server_fn`, ensure
> the LiveRegion component renders the container element during SSR (not deferred to
> client).

## 10. Library Parity

> Compared against: React Aria (`LiveAnnouncer`).

### 10.1 Props

| Feature    | ars-ui                       | React Aria                 | Notes                                        |
| ---------- | ---------------------------- | -------------------------- | -------------------------------------------- |
| Politeness | `politeness: AriaPoliteness` | via `announce()` parameter | Both libraries support polite/assertive      |
| Atomic     | `atomic`                     | --                         | ars-ui addition                              |
| Relevant   | `relevant`                   | --                         | ars-ui addition for `aria-relevant`          |
| Delay      | `delay`                      | --                         | ars-ui addition for clear-then-insert timing |

**Gaps:** None.

### 10.2 Anatomy

| Part | ars-ui                     | React Aria           | Notes                       |
| ---- | -------------------------- | -------------------- | --------------------------- |
| Root | `Root` (`<div aria-live>`) | (hidden live region) | Both use hidden live region |

**Gaps:** None.

### 10.3 Features

| Feature                | ars-ui | React Aria                   |
| ---------------------- | ------ | ---------------------------- |
| Polite/Assertive modes | Yes    | Yes                          |
| Two-step clear/insert  | Yes    | Yes                          |
| Queue management       | Yes    | --                           |
| Configurable delay     | Yes    | --                           |
| Declarative component  | Yes    | -- (imperative `announce()`) |

**Gaps:** None.

### 10.4 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria exposes `announce()` as an imperative function; ars-ui provides a declarative component with state machine. ars-ui adds `atomic`, `relevant`, queue management, and configurable clear-then-insert delay.
- **Recommended additions:** None.
