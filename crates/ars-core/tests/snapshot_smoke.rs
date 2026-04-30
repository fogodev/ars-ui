//! Snapshot coverage for the spec-defined `connect()` / `AttrMap` pattern.

use ars_core::{
    AriaAttr, AttrMap, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, Machine, NoEffect,
    TransitionPlan,
};
use insta::assert_snapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SnapshotState {
    Idle,
    Pressed,
}

#[derive(Clone, Copy, Debug)]
enum SnapshotEvent {
    Toggle,
}

#[derive(Clone, Debug)]
struct SnapshotContext {
    disabled: bool,
    label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SnapshotProps {
    id: String,
}

impl HasId for SnapshotProps {
    fn id(&self) -> &str {
        &self.id
    }

    fn with_id(self, id: String) -> Self {
        Self { id }
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

#[derive(ComponentPart)]
#[scope = "snapshot-toggle"]
enum SnapshotPart {
    Root,
    Indicator,
}

struct SnapshotApi<'a> {
    state: &'a SnapshotState,
    context: &'a SnapshotContext,
    props: &'a SnapshotProps,
}

impl ConnectApi for SnapshotApi<'_> {
    type Part = SnapshotPart;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        let mut attrs = AttrMap::new();

        for (attr, value) in part.data_attrs() {
            attrs.set(attr, value);
        }

        let state_value = match self.state {
            SnapshotState::Idle => "idle",
            SnapshotState::Pressed => "pressed",
        };

        attrs.set(HtmlAttr::Data("ars-state"), state_value);

        match part {
            SnapshotPart::Root => {
                attrs
                    .set(HtmlAttr::Id, self.props.id())
                    .set(HtmlAttr::Role, "button")
                    .set(
                        HtmlAttr::TabIndex,
                        if self.context.disabled { "-1" } else { "0" },
                    )
                    .set(HtmlAttr::Aria(AriaAttr::Label), self.context.label.as_str())
                    .set(
                        HtmlAttr::Aria(AriaAttr::Pressed),
                        match self.state {
                            SnapshotState::Idle => "false",
                            SnapshotState::Pressed => "true",
                        },
                    );

                if self.context.disabled {
                    attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
                }
            }

            SnapshotPart::Indicator => {
                attrs
                    .set(HtmlAttr::Id, format!("{}-indicator", self.props.id()))
                    .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
            }
        }

        attrs
    }
}

struct SnapshotMachine;

impl Machine for SnapshotMachine {
    type State = SnapshotState;
    type Event = SnapshotEvent;
    type Context = SnapshotContext;
    type Props = SnapshotProps;
    type Messages = ();
    type Effect = NoEffect;
    type Api<'a> = SnapshotApi<'a>;

    fn init(
        _props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            SnapshotState::Idle,
            SnapshotContext {
                disabled: false,
                label: String::from("Example toggle"),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (SnapshotState::Idle, SnapshotEvent::Toggle) => {
                Some(TransitionPlan::to(SnapshotState::Pressed))
            }

            (SnapshotState::Pressed, SnapshotEvent::Toggle) => {
                Some(TransitionPlan::to(SnapshotState::Idle))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        SnapshotApi {
            state,
            context,
            props,
        }
    }
}

fn snapshot_attrs(state: SnapshotState, disabled: bool, part: SnapshotPart) -> String {
    let context = SnapshotContext {
        disabled,
        label: String::from("Example toggle"),
    };

    let props = SnapshotProps {
        id: String::from("example-toggle"),
    };

    let api = SnapshotMachine::connect(
        &state,
        &context,
        &props,
        &|event: SnapshotEvent| match event {
            SnapshotEvent::Toggle => {}
        },
    );

    format!("{:#?}", api.part_attrs(part))
}

#[test]
fn snapshot_root_idle_attrs() {
    assert_snapshot!(
        "snapshot_root_idle",
        snapshot_attrs(SnapshotState::Idle, false, SnapshotPart::Root)
    );
}

#[test]
fn snapshot_root_pressed_attrs() {
    assert_snapshot!(
        "snapshot_root_pressed",
        snapshot_attrs(SnapshotState::Pressed, false, SnapshotPart::Root)
    );
}

#[test]
fn snapshot_indicator_pressed_attrs() {
    assert_snapshot!(
        "snapshot_indicator_pressed",
        snapshot_attrs(SnapshotState::Pressed, true, SnapshotPart::Indicator)
    );
}

#[test]
fn snapshot_fixture_machine_toggles_between_states() {
    let next = SnapshotMachine::transition(
        &SnapshotState::Idle,
        &SnapshotEvent::Toggle,
        &SnapshotContext {
            disabled: false,
            label: String::from("Example toggle"),
        },
        &SnapshotProps {
            id: String::from("example-toggle"),
        },
    )
    .expect("toggle event should produce a transition");

    assert_eq!(next.target, Some(SnapshotState::Pressed));
}
