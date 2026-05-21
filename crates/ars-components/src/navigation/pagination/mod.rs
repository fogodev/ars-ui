//! Pagination navigation component.
//!
//! Pagination owns page-state transitions, range generation with ellipses,
//! localized control labels, button-vs-link trigger attributes, and page-change
//! effect intents for adapters.

use alloc::{
    format,
    string::{String, ToString as _},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    num::NonZeroU32,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan, sanitize_url,
};

/// The only pagination machine state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// Current page is tracked in context.
    #[default]
    Idle,
}

/// Events accepted by the pagination state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Navigate to a specific one-based page number.
    GoToPage(u32),

    /// Advance to the next page.
    NextPage,

    /// Move to the previous page.
    PrevPage,

    /// Jump to page one.
    GoToFirstPage,

    /// Jump to the final page.
    GoToLastPage,

    /// Change the page size and clamp the current page.
    SetPageSize(NonZeroU32),

    /// Synchronize render props into context.
    SyncProps,
}

/// Typed effect intents emitted by the pagination machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// The current page changed.
    PageChange,
}

/// Visual size variant for pagination controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Size {
    /// Compact controls for dense layouts.
    Compact,

    /// Standard controls.
    #[default]
    Medium,

    /// Large controls for touch-oriented layouts.
    Large,
}

impl Size {
    /// Returns the rendered size token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

/// Label template for a page-number trigger.
pub type PageLabelFn = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// Announcement template for page changes.
pub type PageChangeAnnouncementFn = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

/// Localized pagination messages.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the root navigation landmark.
    pub root_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the previous-page trigger.
    pub prev_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the next-page trigger.
    pub next_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label template for a page-number trigger.
    pub page_label: MessageFn<PageLabelFn>,

    /// Announcement template for page changes.
    pub page_change_announcement: MessageFn<PageChangeAnnouncementFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            root_label: MessageFn::static_str("Pagination"),
            prev_label: MessageFn::static_str("Go to previous page"),
            next_label: MessageFn::static_str("Go to next page"),
            page_label: MessageFn::new(|page: usize, _locale: &Locale| format!("Page {page}")),
            page_change_announcement: MessageFn::new(Arc::new(
                |current: usize, total: usize, _locale: &Locale| {
                    format!("Page {current} of {total}")
                },
            )
                as Arc<PageChangeAnnouncementFn>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Immutable configuration for a [`Pagination`](self) instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled current page.
    pub page: Option<u32>,

    /// Initial uncontrolled page.
    pub default_page: u32,

    /// Number of items per page.
    pub page_size: NonZeroU32,

    /// Total item count.
    pub total_items: u32,

    /// Number of page buttons shown on each side of the current page.
    pub sibling_count: u32,

    /// Number of always-visible page buttons at each boundary.
    pub boundary_count: u32,

    /// Visual size token.
    pub size: Size,

    /// Optional one-based page URL generator.
    pub get_page_url: Option<Callback<dyn Fn(u32) -> String + Send + Sync>>,

    /// Callback fired by the page-change effect.
    pub on_page_change: Option<Callback<dyn Fn(u32) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            page: None,
            default_page: 1,
            page_size: NonZeroU32::new(10).expect("literal is non-zero"),
            total_items: 0,
            sibling_count: 1,
            boundary_count: 1,
            size: Size::default(),
            get_page_url: None,
            on_page_change: None,
        }
    }
}

impl Props {
    /// Returns default pagination props.
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

    /// Sets [`page`](Self::page).
    #[must_use]
    pub const fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Clears [`page`](Self::page), switching to uncontrolled mode.
    #[must_use]
    pub const fn uncontrolled(mut self) -> Self {
        self.page = None;
        self
    }

    /// Sets [`default_page`](Self::default_page).
    #[must_use]
    pub const fn default_page(mut self, page: u32) -> Self {
        self.default_page = page;
        self
    }

    /// Sets [`page_size`](Self::page_size).
    #[must_use]
    pub const fn page_size(mut self, page_size: NonZeroU32) -> Self {
        self.page_size = page_size;
        self
    }

    /// Sets [`total_items`](Self::total_items).
    #[must_use]
    pub const fn total_items(mut self, count: u32) -> Self {
        self.total_items = count;
        self
    }

    /// Sets [`sibling_count`](Self::sibling_count).
    #[must_use]
    pub const fn sibling_count(mut self, count: u32) -> Self {
        self.sibling_count = count;
        self
    }

    /// Sets [`boundary_count`](Self::boundary_count).
    #[must_use]
    pub const fn boundary_count(mut self, count: u32) -> Self {
        self.boundary_count = count;
        self
    }

    /// Sets [`size`](Self::size).
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets [`get_page_url`](Self::get_page_url).
    #[must_use]
    pub fn get_page_url(
        mut self,
        callback: impl Into<Callback<dyn Fn(u32) -> String + Send + Sync>>,
    ) -> Self {
        self.get_page_url = Some(callback.into());
        self
    }

    /// Sets [`on_page_change`](Self::on_page_change).
    #[must_use]
    pub fn on_page_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(u32) + Send + Sync>>,
    ) -> Self {
        self.on_page_change = Some(callback.into());
        self
    }
}

/// Runtime context for the pagination machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current one-based page.
    pub page: Bindable<u32>,

    /// Number of items per page.
    pub page_size: NonZeroU32,

    /// Total item count.
    pub total_items: u32,

    /// Number of sibling pages shown around the current page.
    pub sibling_count: u32,

    /// Number of boundary pages shown at each edge.
    pub boundary_count: u32,

    /// Derived page count, always at least one.
    pub page_count: u32,

    /// Stable ids derived from props.
    pub ids: ComponentIds,

    /// Active locale.
    pub locale: Locale,

    /// Localized messages.
    pub messages: Messages,
}

impl Context {
    /// Computes `ceil(total_items / page_size)`, clamped to at least one.
    #[must_use]
    pub fn compute_page_count(total_items: u32, page_size: NonZeroU32) -> u32 {
        let size = page_size.get();

        total_items.div_ceil(size).max(1)
    }

    /// Generates the visible one-based page range. `None` represents an ellipsis.
    #[must_use]
    pub fn page_range(&self) -> Vec<Option<u32>> {
        page_range(
            *self.page.get(),
            self.page_count,
            self.sibling_count,
            self.boundary_count,
        )
    }
}

/// Anatomy parts exposed by the pagination connect API.
#[derive(ComponentPart)]
#[scope = "pagination"]
pub enum Part {
    /// Root navigation landmark.
    Root,

    /// Previous-page trigger.
    PrevTrigger,

    /// Next-page trigger.
    NextTrigger,

    /// Numbered page trigger.
    PageTrigger {
        /// One-based page number represented by this trigger.
        #[part(default = 1)]
        page_number: u32,
    },

    /// Non-interactive skipped page range marker.
    Ellipsis,
}

/// Pagination state machine.
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
        let page_count = Context::compute_page_count(props.total_items, props.page_size);

        let initial_page = clamp_page(props.page.unwrap_or(props.default_page), page_count);

        let page = if props.page.is_some() {
            Bindable::controlled(initial_page)
        } else {
            Bindable::uncontrolled(initial_page)
        };

        (
            State::Idle,
            Context {
                page,
                page_size: props.page_size,
                total_items: props.total_items,
                sibling_count: props.sibling_count,
                boundary_count: props.boundary_count,
                page_count,
                ids: ComponentIds::from_id(&props.id),
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        let current = *ctx.page.get();

        match event {
            Event::GoToPage(page) => {
                page_change_plan(ctx, props, clamp_page(*page, ctx.page_count))
            }

            Event::NextPage => page_change_plan(
                ctx,
                props,
                clamp_page(current.saturating_add(1), ctx.page_count),
            ),

            Event::PrevPage => page_change_plan(
                ctx,
                props,
                clamp_page(current.saturating_sub(1), ctx.page_count),
            ),

            Event::GoToFirstPage => page_change_plan(ctx, props, 1),

            Event::GoToLastPage => page_change_plan(ctx, props, ctx.page_count),

            Event::SetPageSize(page_size) => {
                let new_size = *page_size;

                let new_count = Context::compute_page_count(ctx.total_items, new_size);
                let target = clamp_page(current, new_count);
                let emit = target != current;

                let mut plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.page_size = new_size;
                    ctx.page_count = new_count;

                    if ctx.page.is_controlled() {
                        ctx.page.sync_controlled(Some(target));
                    }

                    ctx.page.set(target);
                });

                if emit {
                    plan = with_page_change_effect(plan, target);
                }

                Some(plan)
            }
            Event::SyncProps => {
                let page_size = props.page_size;
                let total_items = props.total_items;
                let sibling_count = props.sibling_count;
                let boundary_count = props.boundary_count;

                let new_count = Context::compute_page_count(total_items, page_size);
                let controlled = props.page.map(|page| clamp_page(page, new_count));
                let target = controlled.unwrap_or_else(|| clamp_page(current, new_count));

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.page_size = page_size;
                    ctx.total_items = total_items;
                    ctx.sibling_count = sibling_count;
                    ctx.boundary_count = boundary_count;
                    ctx.page_count = new_count;
                    ctx.page.sync_controlled(controlled);
                    ctx.page.set(target);
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.page != new.page
            || old.default_page != new.default_page
            || old.page_size != new.page_size
            || old.total_items != new.total_items
            || old.sibling_count != new.sibling_count
            || old.boundary_count != new.boundary_count
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Connected API for a [`Pagination`](self) service.
pub struct Api<'a> {
    /// Current state.
    state: &'a State,

    /// Current context.
    ctx: &'a Context,

    /// Current props.
    props: &'a Props,

    /// Event dispatcher.
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns the current one-based page.
    #[must_use]
    pub fn current_page(&self) -> u32 {
        *self.ctx.page.get()
    }

    /// Returns the total page count.
    #[must_use]
    pub const fn page_count(&self) -> u32 {
        self.ctx.page_count
    }

    /// Returns `true` on the first page.
    #[must_use]
    pub fn is_first_page(&self) -> bool {
        self.current_page() == 1
    }

    /// Returns `true` on the last page.
    #[must_use]
    pub fn is_last_page(&self) -> bool {
        self.current_page() == self.page_count()
    }

    /// Returns the current page range.
    #[must_use]
    pub fn page_range(&self) -> Vec<Option<u32>> {
        self.ctx.page_range()
    }

    /// Returns the current state.
    #[must_use]
    pub const fn state(&self) -> State {
        *self.state
    }

    /// Attributes for the root navigation landmark.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Role, "navigation")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.root_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Data("ars-size"), self.props.size.as_str());

        attrs
    }

    /// Attributes for the previous-page trigger.
    #[must_use]
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = self.trigger_attrs(&Part::PrevTrigger);

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.prev_label)(&self.ctx.locale),
        );

        if self.is_first_page() {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for the next-page trigger.
    #[must_use]
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = self.trigger_attrs(&Part::NextTrigger);

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.next_label)(&self.ctx.locale),
        );

        if self.is_last_page() {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for a numbered page trigger.
    #[must_use]
    pub fn page_trigger_attrs(&self, page_number: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] =
            (Part::PageTrigger { page_number }).data_attrs();

        let page_number = clamp_page(page_number, self.ctx.page_count);

        let current = self.current_page() == page_number;

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Data("ars-index"), page_number.to_string())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.page_label)(page_number as usize, &self.ctx.locale),
            );

        if let Some(get_url) = &self.props.get_page_url {
            let href = get_url(page_number);

            attrs.set(HtmlAttr::Href, sanitize_url(&href));
        } else {
            attrs.set(HtmlAttr::Type, "button");
        }

        if current {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Current), "page")
                .set_bool(HtmlAttr::Data("ars-current"), true);
        }

        attrs
    }

    /// Attributes for an ellipsis marker.
    #[must_use]
    pub fn ellipsis_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Ellipsis.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Role, "separator");

        attrs
    }

    /// Dispatches previous-page navigation when not already at the first page.
    pub fn on_prev_trigger_click(&self) {
        if !self.is_first_page() {
            (self.send)(Event::PrevPage);
        }
    }

    /// Dispatches next-page navigation when not already at the last page.
    pub fn on_next_trigger_click(&self) {
        if !self.is_last_page() {
            (self.send)(Event::NextPage);
        }
    }

    /// Dispatches page navigation for a numbered trigger.
    pub fn on_page_trigger_click(&self, page_number: u32) {
        (self.send)(Event::GoToPage(page_number));
    }

    fn trigger_attrs(&self, part: &Part) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Type, "button");

        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
            Part::PageTrigger { page_number } => self.page_trigger_attrs(page_number),
            Part::Ellipsis => self.ellipsis_attrs(),
        }
    }
}

fn clamp_page(page: u32, page_count: u32) -> u32 {
    page.max(1).min(page_count.max(1))
}

fn page_change_plan(ctx: &Context, _props: &Props, target: u32) -> Option<TransitionPlan<Machine>> {
    if target == *ctx.page.get() {
        return None;
    }

    Some(with_page_change_effect(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.page.set(target);
        }),
        target,
    ))
}

fn with_page_change_effect(
    mut plan: TransitionPlan<Machine>,
    target: u32,
) -> TransitionPlan<Machine> {
    plan = plan.with_effect(PendingEffect::new(
        Effect::PageChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_page_change {
                (callback)(target);
            }

            ars_core::no_cleanup()
        },
    ));

    plan
}

fn page_range(
    current_page: u32,
    page_count: u32,
    sibling_count: u32,
    boundary_count: u32,
) -> Vec<Option<u32>> {
    let total = page_count.max(1);

    let current = clamp_page(current_page, total);

    let boundary = boundary_count;

    let siblings = sibling_count;

    let all_count = boundary
        .saturating_mul(2)
        .saturating_add(siblings.saturating_mul(2))
        .saturating_add(3);

    if total <= all_count {
        return (1..=total).map(Some).collect();
    }

    let left_boundary_end = boundary.min(total);
    let right_boundary_start = if boundary == 0 {
        total
    } else {
        total.saturating_sub(boundary).saturating_add(1).max(1)
    };

    let middle_min = left_boundary_end.saturating_add(1);
    let middle_max = if boundary == 0 {
        total
    } else {
        right_boundary_start.saturating_sub(1)
    };

    let sibling_start = current
        .saturating_sub(siblings)
        .max(middle_min)
        .min(middle_max);
    let sibling_end = current
        .saturating_add(siblings)
        .min(middle_max)
        .max(sibling_start);

    let mut pages = Vec::new();

    for page in 1..=left_boundary_end {
        pages.push(Some(page));
    }

    if sibling_start > middle_min {
        pages.push(None);
    } else {
        for page in middle_min..sibling_start {
            pages.push(Some(page));
        }
    }

    for page in sibling_start..=sibling_end {
        pages.push(Some(page));
    }

    if sibling_end < middle_max {
        pages.push(None);
    } else {
        for page in sibling_end.saturating_add(1)..=middle_max {
            pages.push(Some(page));
        }
    }

    if boundary > 0 {
        for page in right_boundary_start..=total {
            if pages.last() != Some(&Some(page)) {
                pages.push(Some(page));
            }
        }
    }

    pages
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString as _, sync::Arc, vec};
    use std::sync::{Mutex, MutexGuard};

    use ars_core::{Service, StrongSend};

    use super::*;

    fn props() -> Props {
        Props::new()
            .id("pager")
            .total_items(100)
            .page_size(NonZeroU32::new(10).expect("non-zero"))
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().expect("test mutex should not be poisoned")
    }

    #[test]
    fn computes_page_count_and_range_with_ellipsis() {
        let service = service(props().page(5).sibling_count(1).boundary_count(1));

        let api = service.connect(&|_| {});

        assert_eq!(api.page_count(), 10);
        assert_eq!(
            api.page_range(),
            vec![Some(1), None, Some(4), Some(5), Some(6), None, Some(10)]
        );
    }

    #[test]
    fn page_range_handles_boundary_adjacency_cases() {
        assert_eq!(
            page_range(2, 10, 1, 1),
            vec![Some(1), Some(2), Some(3), None, Some(10)]
        );
        assert_eq!(
            page_range(9, 10, 1, 1),
            vec![Some(1), None, Some(8), Some(9), Some(10)]
        );
        assert_eq!(
            page_range(5, 12, 1, 2),
            vec![
                Some(1),
                Some(2),
                None,
                Some(4),
                Some(5),
                Some(6),
                None,
                Some(11),
                Some(12),
            ]
        );
        assert_eq!(
            page_range(4, 10, 1, 2),
            vec![
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                None,
                Some(9),
                Some(10),
            ]
        );
        assert_eq!(
            page_range(7, 10, 1, 2),
            vec![
                Some(1),
                Some(2),
                None,
                Some(6),
                Some(7),
                Some(8),
                Some(9),
                Some(10),
            ]
        );
    }

    #[test]
    fn page_range_honors_zero_boundary_count() {
        let range = page_range(5, 10, 1, 0);

        assert_eq!(range, vec![None, Some(4), Some(5), Some(6), None]);
        assert!(!range.contains(&Some(1)));
        assert!(!range.contains(&Some(10)));
    }

    #[test]
    fn prev_next_and_current_attrs_match_spec() {
        let service = service(props().default_page(1));

        let api = service.connect(&|_| {});

        insta::assert_snapshot!("pagination_root", snapshot_attrs(&api.root_attrs()));
        insta::assert_snapshot!(
            "pagination_prev_disabled",
            snapshot_attrs(&api.prev_trigger_attrs())
        );
        insta::assert_snapshot!(
            "pagination_next_enabled",
            snapshot_attrs(&api.next_trigger_attrs())
        );
        insta::assert_snapshot!(
            "pagination_page_current",
            snapshot_attrs(&api.page_trigger_attrs(1))
        );
        insta::assert_snapshot!("pagination_ellipsis", snapshot_attrs(&api.ellipsis_attrs()));
    }

    #[test]
    fn last_page_disables_next_trigger() {
        let service = service(props().default_page(10));

        let attrs = service.connect(&|_| {}).next_trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn page_trigger_uses_sanitized_url_when_generator_exists() {
        let service = service(props().get_page_url(|page| {
            if page == 2 {
                "javascript:alert(1)".to_string()
            } else {
                format!("/page/{page}")
            }
        }));

        let api = service.connect(&|_| {});

        assert_eq!(
            api.page_trigger_attrs(3).get(&HtmlAttr::Href),
            Some("/page/3")
        );
        assert_eq!(api.page_trigger_attrs(2).get(&HtmlAttr::Href), Some("#"));
        assert!(api.page_trigger_attrs(2).get(&HtmlAttr::Type).is_none());
    }

    #[test]
    fn page_navigation_clamps_at_bounds() {
        let mut service = service(props().default_page(1));

        assert!(service.send(Event::PrevPage).pending_effects.is_empty());
        assert_eq!(*service.context().page.get(), 1);

        drop(service.send(Event::NextPage));

        assert_eq!(*service.context().page.get(), 2);

        drop(service.send(Event::GoToPage(99)));

        assert_eq!(*service.context().page.get(), 10);

        assert!(service.send(Event::NextPage).pending_effects.is_empty());
        assert_eq!(*service.context().page.get(), 10);
    }

    #[test]
    fn next_page_at_u32_max_page_count_is_noop() {
        let mut service = service(
            Props::new()
                .id("pager")
                .page(u32::MAX)
                .total_items(u32::MAX)
                .page_size(NonZeroU32::new(1).expect("non-zero")),
        );

        let result = service.send(Event::NextPage);

        assert!(result.pending_effects.is_empty());
        assert!(!result.context_changed);
        assert_eq!(*service.context().page.get(), u32::MAX);
    }

    #[test]
    fn page_change_effect_runs_callback() {
        let pages = Arc::new(Mutex::new(Vec::new()));
        let pages_clone = Arc::clone(&pages);
        let mut service =
            service(props().on_page_change(move |page| lock(&pages_clone).push(page)));

        let result = service.send(Event::GoToPage(3));

        assert_eq!(result.pending_effects[0].name, Effect::PageChange);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(lock(&pages).as_slice(), &[3]);
    }

    #[test]
    fn controlled_page_change_callback_receives_requested_target() {
        let pages = Arc::new(Mutex::new(Vec::new()));
        let pages_clone = Arc::clone(&pages);
        let mut service = service(
            props()
                .page(2)
                .on_page_change(move |page| lock(&pages_clone).push(page)),
        );

        let result = service.send(Event::GoToPage(4));

        assert_eq!(*service.context().page.get(), 2);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(lock(&pages).as_slice(), &[4]);
    }

    #[test]
    fn controlled_initial_page_is_clamped_to_page_count() {
        let service = service(props().page(20));

        assert_eq!(service.context().page_count, 10);
        assert_eq!(*service.context().page.get(), 10);

        let api = service.connect(&|_| {});

        assert_eq!(api.current_page(), 10);
        assert!(api.is_last_page());
    }

    #[test]
    fn controlled_set_page_size_clamps_visible_page_when_page_count_shrinks() {
        let mut service = service(props().page(10));

        let result = service.send(Event::SetPageSize(
            NonZeroU32::new(20).expect("non-zero page size"),
        ));

        assert_eq!(service.context().page_count, 5);
        assert_eq!(*service.context().page.get(), 5);
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::PageChange);

        let api = service.connect(&|_| {});

        assert_eq!(api.current_page(), 5);
        assert!(api.is_last_page());
    }

    #[test]
    fn sync_props_clamps_controlled_page_to_new_page_count() {
        let mut service = service(props().page(4));

        service.set_props(props().page(8).total_items(25));

        drop(service.send(Event::SyncProps));

        assert_eq!(service.context().page_count, 3);
        assert_eq!(*service.context().page.get(), 3);

        let api = service.connect(&|_| {});

        assert_eq!(api.current_page(), 3);
        assert!(api.is_last_page());
    }

    #[test]
    fn controlled_page_change_callback_keeps_requested_target_after_clamped_size_change() {
        let pages = Arc::new(Mutex::new(Vec::new()));
        let pages_clone = Arc::clone(&pages);

        let mut service = service(
            props()
                .page(10)
                .on_page_change(move |page| lock(&pages_clone).push(page)),
        );

        let result = service.send(Event::SetPageSize(
            NonZeroU32::new(20).expect("non-zero page size"),
        ));

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(lock(&pages).as_slice(), &[5]);
    }

    #[test]
    fn api_handlers_dispatch_navigation_events() {
        let sent = core::cell::RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let service = service(props().default_page(2));

        let api = service.connect(&send);

        api.on_prev_trigger_click();
        api.on_next_trigger_click();
        api.on_page_trigger_click(4);

        assert_eq!(
            sent.borrow().as_slice(),
            &[Event::PrevPage, Event::NextPage, Event::GoToPage(4)]
        );
    }

    #[test]
    fn set_page_size_clamps_and_emits_only_when_page_changes() {
        let mut service = service(props().default_page(10));

        let result = service.send(Event::SetPageSize(
            NonZeroU32::new(20).expect("non-zero page size"),
        ));

        assert_eq!(*service.context().page.get(), 5);
        assert_eq!(service.context().page_count, 5);
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::PageChange);

        let result = service.send(Event::SetPageSize(
            NonZeroU32::new(25).expect("non-zero page size"),
        ));

        assert_eq!(*service.context().page.get(), 4);
        assert_eq!(service.context().page_count, 4);
        assert_eq!(result.pending_effects.len(), 1);

        let result = service.send(Event::SetPageSize(
            NonZeroU32::new(10).expect("non-zero page size"),
        ));

        assert_eq!(*service.context().page.get(), 4);
        assert_eq!(service.context().page_count, 10);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn on_props_changed_detects_each_context_field() {
        let old = props()
            .page(2)
            .default_page(3)
            .sibling_count(1)
            .boundary_count(1);

        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &old).is_empty());

        for new in [
            old.clone().page(4),
            old.clone().default_page(4),
            old.clone()
                .page_size(NonZeroU32::new(20).expect("non-zero page size")),
            old.clone().total_items(200),
            old.clone().sibling_count(2),
            old.clone().boundary_count(2),
        ] {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                [Event::SyncProps],
                "expected SyncProps for {new:?}"
            );
        }
    }

    #[test]
    fn part_attrs_dispatches_every_part() {
        let service = service(props().default_page(2));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::PrevTrigger), api.prev_trigger_attrs());
        assert_eq!(api.part_attrs(Part::NextTrigger), api.next_trigger_attrs());
        assert_eq!(
            api.part_attrs(Part::PageTrigger { page_number: 2 }),
            api.page_trigger_attrs(2)
        );
        assert_eq!(api.part_attrs(Part::Ellipsis), api.ellipsis_attrs());
    }
}
