//! `ColorSwatchPicker` component state machine and connect API.
//!
//! `ColorSwatchPicker` is a listbox of selectable color swatches. It owns the
//! color list, selection state, roving focus, keyboard navigation (grid or
//! stack), and ARIA/data attributes. Each item embeds a
//! [`color_swatch`](crate::specialized::color_swatch) for its visual sample and
//! accessible name; the adapter builds that swatch from
//! [`Api::item_swatch_props`]. Live swatch measurement and focus movement are
//! adapter concerns driven from the events this machine accepts.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ColorValue, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

use crate::specialized::color_swatch;

/// Label for the picker root.
type LabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Layout mode for [`ColorSwatchPicker`](self).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SwatchPickerLayout {
    /// 2D grid with configurable columns. Default.
    #[default]
    Grid,

    /// 1D horizontal or vertical stack.
    Stack,
}

/// The states for the `ColorSwatchPicker` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No focus within the picker.
    Idle,

    /// A swatch item is focused.
    Focused,
}

/// The events for the `ColorSwatchPicker` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus entered the picker.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus left the picker.
    Blur,

    /// Select the color at the given index.
    Select {
        /// The index of the color to select.
        index: usize,
    },

    /// Navigate to the next swatch.
    FocusNext,

    /// Navigate to the previous swatch.
    FocusPrev,

    /// Grid-mode: navigate to the swatch in the row above.
    FocusUp,

    /// Grid-mode: navigate to the swatch in the row below.
    FocusDown,

    /// Jump to the first swatch.
    FocusFirst,

    /// Jump to the last swatch.
    FocusLast,
}

/// The context for the `ColorSwatchPicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The currently selected color.
    pub value: Bindable<ColorValue>,

    /// Index of the currently focused swatch, if any.
    pub focused_index: Option<usize>,

    /// The list of colors in the picker.
    pub colors: Vec<ColorValue>,

    /// Layout mode (grid or stack).
    pub layout: SwatchPickerLayout,

    /// Number of columns for grid layout.
    pub columns: usize,

    /// Whether the picker is disabled.
    pub disabled: bool,

    /// Whether focus is within the picker.
    pub focused: bool,

    /// Whether focus was via keyboard (for the focus-visible ring).
    pub focus_visible: bool,

    /// Locale for internationalized messages.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// The props for the `ColorSwatchPicker` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,

    /// Default value for uncontrolled mode.
    pub default_value: ColorValue,

    /// The list of colors to display as swatches.
    pub colors: Vec<ColorValue>,

    /// Layout mode. Default: `Grid`.
    pub layout: SwatchPickerLayout,

    /// Number of columns for grid layout. Default: `5`.
    pub columns: usize,

    /// Whether the picker is disabled.
    pub disabled: bool,

    /// Name attribute for the hidden form input.
    pub name: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: ColorValue::default(),
            colors: Vec::new(),
            layout: SwatchPickerLayout::Grid,
            columns: 5,
            disabled: false,
            name: None,
        }
    }
}

/// The messages for the `ColorSwatchPicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the picker root. Default: `"Color swatches"`.
    pub label: MessageFn<LabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Color swatches"),
        }
    }
}

impl ComponentMessages for Messages {}

/// The machine for the `ColorSwatchPicker` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let value = if let Some(v) = &props.value {
            Bindable::controlled(*v)
        } else {
            Bindable::uncontrolled(props.default_value)
        };

        let context = Context {
            value,
            focused_index: None,
            colors: props.colors.clone(),
            layout: props.layout,
            columns: props.columns,
            disabled: props.disabled,
            focused: false,
            focus_visible: false,
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, context)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled || ctx.colors.is_empty() {
            return None;
        }

        match event {
            Event::Focus { is_keyboard } => {
                let kb = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = kb;

                        // Focus the selected item, or the first item.
                        if ctx.focused_index.is_none() {
                            let value = *ctx.value.get();
                            let selected =
                                ctx.colors.iter().position(|candidate| *candidate == value);
                            ctx.focused_index = Some(selected.unwrap_or(0));
                        }
                    }),
                )
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            Event::Select { index } => {
                let idx = *index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if idx < ctx.colors.len() {
                        ctx.value.set(ctx.colors[idx]);
                        ctx.focused_index = Some(idx);
                    }
                }))
            }

            Event::FocusNext => {
                if !matches!(state, State::Focused) {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let current = ctx.focused_index.unwrap_or(0);
                    ctx.focused_index = Some((current + 1) % ctx.colors.len());
                }))
            }

            Event::FocusPrev => {
                if !matches!(state, State::Focused) {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let len = ctx.colors.len();
                    let current = ctx.focused_index.unwrap_or(0);
                    ctx.focused_index = Some((current + len - 1) % len);
                }))
            }

            Event::FocusUp => {
                if !matches!(state, State::Focused) || ctx.layout != SwatchPickerLayout::Grid {
                    return None;
                }

                let cols = ctx.columns;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let len = ctx.colors.len();
                    let current = ctx.focused_index.unwrap_or(0);
                    ctx.focused_index = Some((current + len - cols % len) % len);
                }))
            }

            Event::FocusDown => {
                if !matches!(state, State::Focused) || ctx.layout != SwatchPickerLayout::Grid {
                    return None;
                }

                let cols = ctx.columns;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let current = ctx.focused_index.unwrap_or(0);
                    ctx.focused_index = Some((current + cols) % ctx.colors.len());
                }))
            }

            Event::FocusFirst => {
                if !matches!(state, State::Focused) {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused_index = Some(0);
                }))
            }

            Event::FocusLast => {
                if !matches!(state, State::Focused) {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused_index = Some(ctx.colors.len() - 1);
                }))
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
}

/// Structural parts exposed by the `ColorSwatchPicker` connect API.
#[derive(ComponentPart)]
#[scope = "color-swatch-picker"]
pub enum Part {
    /// Container with `role="listbox"`.
    Root,

    /// A selectable swatch item with `role="option"`, parameterized by index.
    Item {
        /// The item index.
        index: usize,
    },

    /// `type="hidden"` input that submits the hex value for forms.
    HiddenInput,
}

/// The connect API for the `ColorSwatchPicker` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("color_swatch_picker::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// The currently selected color.
    #[must_use]
    pub fn value(&self) -> &ColorValue {
        self.ctx.value.get()
    }

    /// The index of the currently focused swatch, if any.
    #[must_use]
    pub const fn focused_index(&self) -> Option<usize> {
        self.ctx.focused_index
    }

    const fn state_str(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
        }
    }

    /// Attributes for the root listbox element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id().to_string())
            .set(HtmlAttr::Role, "listbox")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Data("ars-state"), self.state_str());

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        // Stack is 1D (horizontal); grid uses 2D navigation and omits orientation.
        if self.ctx.layout == SwatchPickerLayout::Stack {
            attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), "horizontal");
        }

        attrs
    }

    /// Attributes for the item wrapper at the given index.
    #[must_use]
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.item("item", &index))
            .set(HtmlAttr::Role, "option");

        let is_selected =
            index < self.ctx.colors.len() && self.ctx.colors[index] == *self.ctx.value.get();

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Selected),
            if is_selected { "true" } else { "false" },
        );

        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        let is_focused = self.ctx.focused_index == Some(index);

        // Roving tabindex: only the focused item is tabbable.
        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });

        if is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);

            if self.ctx.focus_visible {
                attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
            }
        }

        attrs
    }

    /// Returns the props for the embedded [`color_swatch`] within the item at `index`.
    #[must_use]
    pub fn item_swatch_props(&self, index: usize) -> color_swatch::Props {
        let color = self.ctx.colors.get(index).copied().unwrap_or_default();

        color_swatch::Props {
            id: self.ctx.ids.item("swatch", &index),
            color,
            color_name: None,
            respect_alpha: true,
        }
    }

    /// Attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_hex(true));

        attrs
    }

    /// Handles keyboard navigation on the root/items.
    pub fn on_keydown(&self, data: &KeyboardEventData) {
        let stack = self.ctx.layout == SwatchPickerLayout::Stack;

        match data.key {
            KeyboardKey::ArrowRight => (self.send)(Event::FocusNext),

            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrev),

            KeyboardKey::ArrowDown => (self.send)(if stack {
                Event::FocusNext
            } else {
                Event::FocusDown
            }),

            KeyboardKey::ArrowUp => (self.send)(if stack {
                Event::FocusPrev
            } else {
                Event::FocusUp
            }),

            KeyboardKey::Home => (self.send)(Event::FocusFirst),

            KeyboardKey::End => (self.send)(Event::FocusLast),

            KeyboardKey::Space | KeyboardKey::Enter => {
                if let Some(idx) = self.ctx.focused_index {
                    (self.send)(Event::Select { index: idx });
                }
            }

            _ => {}
        }
    }

    /// Handles a click on the item at `index`.
    pub fn on_item_click(&self, index: usize) {
        (self.send)(Event::Select { index });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use ars_core::Service;
    use insta::assert_snapshot;

    use super::*;

    fn palette() -> Vec<ColorValue> {
        vec![
            ColorValue::from_rgb(255, 0, 0),
            ColorValue::from_rgb(0, 255, 0),
            ColorValue::from_rgb(0, 0, 255),
            ColorValue::from_rgb(255, 255, 0),
            ColorValue::from_rgb(0, 255, 255),
            ColorValue::from_rgb(255, 0, 255),
        ]
    }

    fn service(mut props: Props) -> Service<Machine> {
        if props.id.is_empty() {
            props.id = "color-swatch-picker".to_string();
        }

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        alloc::format!("{attrs:#?}")
    }

    fn key(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    #[test]
    fn root_is_listbox_with_label() {
        let svc = service(Props {
            colors: palette(),
            ..Props::default()
        });

        let root = svc.connect(&|_| {}).root_attrs();

        assert_eq!(root.get(&HtmlAttr::Role), Some("listbox"));
        assert_eq!(
            root.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Color swatches")
        );
    }

    #[test]
    fn stack_layout_sets_horizontal_orientation_grid_does_not() {
        let stack = service(Props {
            colors: palette(),
            layout: SwatchPickerLayout::Stack,
            ..Props::default()
        });

        assert_eq!(
            stack
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );

        let grid = service(Props {
            colors: palette(),
            ..Props::default()
        });

        assert!(
            !grid
                .connect(&|_| {})
                .root_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::Orientation))
        );
    }

    #[test]
    fn item_exposes_option_role_selected_and_roving_tabindex() {
        let svc = service(Props {
            colors: palette(),
            default_value: ColorValue::from_rgb(0, 0, 255),
            ..Props::default()
        });

        let api = svc.connect(&|_| {});

        // Index 2 is blue, the selected value.
        let selected = api.item_attrs(2);

        assert_eq!(selected.get(&HtmlAttr::Role), Some("option"));
        assert_eq!(
            selected.get(&HtmlAttr::Aria(AriaAttr::Selected)),
            Some("true")
        );

        // A non-selected item.
        let other = api.item_attrs(0);

        assert_eq!(
            other.get(&HtmlAttr::Aria(AriaAttr::Selected)),
            Some("false")
        );
        assert_eq!(other.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn selecting_an_item_sets_value() {
        let mut svc = service(Props {
            colors: palette(),
            ..Props::default()
        });

        drop(svc.send(Event::Select { index: 3 }));

        assert_eq!(svc.connect(&|_| {}).value().to_rgb(), (255, 255, 0));
        assert_eq!(svc.connect(&|_| {}).focused_index(), Some(3));
    }

    #[test]
    fn keyboard_navigation_moves_focus_in_grid() {
        let mut svc = service(Props {
            colors: palette(),
            columns: 3,
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_eq!(svc.connect(&|_| {}).focused_index(), Some(0));

        let captured = core::cell::RefCell::new(vec![]);
        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_keydown(&key(KeyboardKey::ArrowDown));

        assert!(matches!(captured.borrow()[0], Event::FocusDown));
    }

    #[test]
    fn grid_focus_down_wraps_by_columns() {
        let mut svc = service(Props {
            colors: palette(),
            columns: 3,
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));
        drop(svc.send(Event::FocusDown));

        // 0 + 3 columns -> index 3.
        assert_eq!(svc.connect(&|_| {}).focused_index(), Some(3));
    }

    #[test]
    fn home_end_jump_to_first_and_last() {
        let mut svc = service(Props {
            colors: palette(),
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));
        drop(svc.send(Event::FocusLast));

        assert_eq!(svc.connect(&|_| {}).focused_index(), Some(5));

        drop(svc.send(Event::FocusFirst));

        assert_eq!(svc.connect(&|_| {}).focused_index(), Some(0));
    }

    #[test]
    fn item_swatch_props_carry_the_color() {
        let svc = service(Props {
            colors: palette(),
            ..Props::default()
        });

        let props = svc.connect(&|_| {}).item_swatch_props(1);

        assert_eq!(props.color.to_rgb(), (0, 255, 0));
        assert_eq!(props.id, "color-swatch-picker-swatch-1");
    }

    #[test]
    fn empty_picker_ignores_events() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_eq!(svc.state(), &State::Idle);
    }

    #[test]
    fn disabled_picker_ignores_events() {
        let mut svc = service(Props {
            colors: palette(),
            disabled: true,
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(
            svc.connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
    }

    #[test]
    fn root_focused_snapshot() {
        let mut svc = service(Props {
            id: "csp".to_string(),
            colors: palette(),
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "color_swatch_picker_root_focused",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn item_selected_snapshot() {
        let svc = service(Props {
            id: "csp".to_string(),
            colors: palette(),
            default_value: ColorValue::from_rgb(0, 255, 0),
            ..Props::default()
        });

        assert_snapshot!(
            "color_swatch_picker_item_selected",
            snapshot_attrs(&svc.connect(&|_| {}).item_attrs(1))
        );
    }

    #[test]
    fn exhaustive_events_parts_and_helpers() {
        let mut svc = Service::<Machine>::new(
            Props {
                id: "csp".into(),
                value: Some(ColorValue::from_rgb(0, 0, 255)),
                colors: palette(),
                columns: 3,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        for ev in [
            Event::Focus { is_keyboard: true },
            Event::FocusNext,
            Event::FocusPrev,
            Event::FocusDown,
            Event::FocusUp,
            Event::FocusFirst,
            Event::FocusLast,
            Event::Select { index: 2 },
            Event::Blur,
        ] {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        for p in [Part::Root, Part::Item { index: 0 }, Part::HiddenInput] {
            let _attrs = api.part_attrs(p);
        }

        let _swatch = api.item_swatch_props(0);
        let _oob = api.item_swatch_props(99); // out-of-bounds -> default color

        let _val = api.value();

        let _dbg = alloc::format!("{api:?}");

        // Stack layout focus events.
        let mut stack = Service::<Machine>::new(
            Props {
                id: "csp".into(),
                colors: palette(),
                layout: SwatchPickerLayout::Stack,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(stack.send(Event::Focus { is_keyboard: true }));
        // FocusUp/Down are grid-only: no-ops in stack layout.
        drop(stack.send(Event::FocusUp));
        drop(stack.send(Event::FocusDown));

        // Navigation events before focus (Idle) are ignored.
        let mut idle = Service::<Machine>::new(
            Props {
                id: "csp".into(),
                colors: palette(),
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(idle.send(Event::FocusNext));

        assert_eq!(idle.connect(&|_| {}).focused_index(), None);

        // Keyboard dispatch helpers (stack + grid + select).
        let cap = core::cell::RefCell::new(vec![]);
        let send = |event: Event| cap.borrow_mut().push(event);

        let dapi = stack.connect(&send);

        dapi.on_keydown(&key(KeyboardKey::ArrowDown)); // stack -> FocusNext
        dapi.on_keydown(&key(KeyboardKey::ArrowUp)); // stack -> FocusPrev
        dapi.on_keydown(&key(KeyboardKey::Home));
        dapi.on_keydown(&key(KeyboardKey::End));
        dapi.on_keydown(&key(KeyboardKey::Enter));
        dapi.on_item_click(2);

        let evs = cap.borrow();

        assert!(matches!(evs[0], Event::FocusNext));
        assert!(matches!(evs[1], Event::FocusPrev));
    }
}
