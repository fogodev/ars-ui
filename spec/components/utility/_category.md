# Utility Components Specification

Cross-references: `00-overview.md` for naming conventions and data attributes,
`01-architecture.md` for the `Machine` trait, `AttrMap`, `Bindable`, and `Service`.
`03-accessibility.md` for focus management, ARIA patterns, and keyboard navigation.
`05-interactions.md` for press, hover, and focus interaction primitives.

---

## Overview

Utility components are the foundational building blocks that enable complex UI patterns. Unlike
visual components, they primarily provide accessibility, focus management, animation lifecycle,
and polymorphic rendering primitives.

These components sit at the lowest layer of the ars-ui stack. Many are used internally by
higher-level components (`Dialog` uses `FocusScope` and `Dismissable`; `Toast` uses `LiveRegion`;
icon-only `Button`s use `VisuallyHidden`). Understanding these primitives is essential before
reading any overlay or form component specification.

---

## Table of Contents

- [ActionGroup](action-group.md)
- [AsChild (Polymorphic Pattern)](as-child.md)
- [Button](button.md)
- [ClientOnly](client-only.md)
- [Dismissable](dismissable.md)
- [DownloadTrigger](download-trigger.md)
- [DropZone](drop-zone.md)
- [ArsProvider](ars-provider.md)
- [Field](field.md)
- [Fieldset](fieldset.md)
- [FocusRing](focus-ring.md)
- [FocusScope](focus-scope.md)
- [Form](form.md)
- [Group](group.md)
- [Heading](heading.md)
- [Highlight](highlight.md)
- [Keyboard](keyboard.md)
- [Landmark](landmark.md)
- [LiveRegion](live-region.md)
- [Separator](separator.md)
- [Swap](swap.md)
- [Toggle](toggle.md)
- [ToggleButton](toggle-button.md)
- [ToggleGroup](toggle-group.md)
- [VisuallyHidden](visually-hidden.md)
- [ZIndexAllocator](z-index-allocator.md)

---

## Summary Table

| Component         | State Machine                 | Key ARIA Pattern                                      | Used Internally By                        |
| ----------------- | ----------------------------- | ----------------------------------------------------- | ----------------------------------------- |
| `ActionGroup`     | Yes (focus, overflow)         | `role="toolbar"`, roving tabindex, overflow menu      | `Action` bars with overflow               |
| `AsChild`         | No                            | Prop merging onto child                               | All components with `as_child`            |
| `Button`          | Yes (loading, focus, pressed) | `role="button"`, `type="button"`                      | `Toggle`, many others                     |
| `ClientOnly`      | No                            | SSR fallback rendering                                | SSR-dependent components                  |
| `Dismissable`     | No                            | Outside-click/Escape dismiss                          | `Dialog`, `Popover`, `Select`, `Menu`     |
| `DownloadTrigger` | No                            | `<a download>` native download                        | File download buttons                     |
| `DropZone`        | Yes (drag-over, drop)         | `aria-description`, `role="button"`                   | `FileUpload`                              |
| `ArsProvider`     | No                            | Context provider (no DOM output)                      | `FocusScope`, SSR                         |
| `Field`           | No                            | `<label>` + error/description association             | Form inputs                               |
| `Fieldset`        | No                            | `<fieldset>` + `<legend>`                             | Form groups                               |
| `FocusRing`       | No                            | `data-ars-focus-visible`                              | All interactive components                |
| `FocusScope`      | Yes (trap activation)         | Focus trap (Tab interception)                         | `Dialog`, `AlertDialog`, `Drawer`         |
| `Form`            | No                            | `<form>` semantic element, validation                 | Form submission and validation            |
| `Group`           | No                            | `role="group"`, state propagation via context         | Non-form grouping, `NumberField` clusters |
| `Heading`         | No                            | `role="heading"`, `aria-level`                        | `Section` headings                        |
| `Highlight`       | No                            | `<mark>` for matches                                  | `Search` results, `Combobox`              |
| `Keyboard`        | No                            | `<kbd>` semantic element                              | `Menu` shortcut hints, shortcut displays  |
| `Landmark`        | No                            | HTML5 landmark elements / `role` fallback             | Page structure                            |
| `LiveRegion`      | Yes (announce timing)         | `aria-live`                                           | `Toast`, `Combobox`, `FileUpload`         |
| `Separator`       | No                            | `role="separator"`                                    | `Menu`, `Toolbar`, `Dividers`             |
| `Swap`            | Yes (swap animation)          | `role="button"`, `aria-pressed`                       | Content swap with animation               |
| `Toggle`          | Yes (on/off, focus)           | `aria-pressed`                                        | `ToggleGroup`                             |
| `ToggleButton`    | Yes (on/off, focus, pressed)  | `aria-pressed`, `role="button"`                       | `ToggleGroup`                             |
| `ToggleGroup`     | Yes (roving focus)            | `role="group"`, `aria-pressed` or `role="radiogroup"` | `Toolbar` patterns                        |
| `VisuallyHidden`  | No                            | CSS clip-rect trick                                   | Icon buttons, labels                      |
| `ZIndexAllocator` | No                            | Z-index layer management                              | Overlays, stacking contexts               |

---

## Code Quality Notes

### Derive Macros

All state, event, context, and props types use the standard derive macros where applicable:

```rust
pub mod button {
    /// The states for the `Button` component.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum State {
        /// Default resting state. Not focused, not pressed.
        Idle,
        /// The button has received focus. Keyboard focus also sets `focus_visible`.
        Focused,
        /// The button is actively being pressed (pointer held down or Space held).
        Pressed,
        /// The button is in a loading state. Interaction is disabled.
        Loading,
    }

    // `button::Variant` and `button::Size` removed — headless design uses Option<String> for both.
    // Consumers supply their own variant/size strings; the core renders them as data-ars-variant / data-ars-size.
}

// selection::Mode { None, Single, Multiple } — defined in ToggleGroup (§1) above.

// `toggle::State` and `toggle::Event` are defined in the Toggle component specification.

// AriaPoliteness — defined in `live-region.md`
// AnnouncePriority — defined in `live-region.md`

// Orientation and Direction are imported from ars_i18n:
// use ars_i18n::{Direction, Orientation};
```

### Visibility

All public API types and methods use `pub`. Internal implementation details use `pub(crate)` or
are private. Event handler closures use `move` captures with cloned send functions.

### Testing

Each machine is fully testable without a DOM:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ars_core::Service;

    // ── Button tests ─────────────────────────────────────────────────────────

    #[test]
    fn button_starts_idle() {
        let props = Props::default();
        let service = Service::<Button>::new(props);
        assert_eq!(*service.state(), State::Idle);
    }

    #[test]
    fn button_focus_keyboard_sets_focus_visible() {
        let props = Props::default();
        let mut service = Service::<Button>::new(props);
        service.send(Event::Focus { is_keyboard: true });
        assert_eq!(*service.state(), State::Focused);
        assert!(service.context().focus_visible);
    }

    #[test]
    fn button_focus_pointer_does_not_set_focus_visible() {
        let props = Props::default();
        let mut service = Service::<Button>::new(props);
        service.send(Event::Focus { is_keyboard: false });
        assert_eq!(*service.state(), State::Focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn button_loading_disables_interaction() {
        let mut props = Props::default();
        props.loading = true;
        let service = Service::<Button>::new(props);
        assert_eq!(*service.state(), State::Loading);
    }

    #[test]
    fn button_set_loading_transitions() {
        let props = Props::default();
        let mut service = Service::<Button>::new(props);
        assert_eq!(*service.state(), State::Idle);
        service.send(Event::SetLoading(true));
        assert_eq!(*service.state(), State::Loading);
        service.send(Event::SetLoading(false));
        assert_eq!(*service.state(), State::Idle);
    }

    #[test]
    fn button_disabled_ignores_events() {
        let mut props = Props::default();
        props.disabled = true;
        let mut service = Service::<Button>::new(props);
        service.send(Event::Focus { is_keyboard: true });
        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().focus_visible);
    }

    // ── Toggle tests ─────────────────────────────────────────────────────────

    #[test]
    fn toggle_starts_off() {
        let props = Props { id: String::new(), pressed: None, default_pressed: false, disabled: false };
        let service = Service::<Toggle>::new(props);
        assert_eq!(*service.state(), State::Off);
    }

    #[test]
    fn toggle_flip_on_off() {
        let props = Props { id: String::new(), pressed: None, default_pressed: false, disabled: false };
        let mut service = Service::<Toggle>::new(props);
        service.send(Event::Toggle);
        assert_eq!(*service.state(), State::On);
        service.send(Event::Toggle);
        assert_eq!(*service.state(), State::Off);
    }

    #[test]
    fn toggle_disabled_ignores_toggle() {
        let props = Props { id: String::new(), pressed: None, default_pressed: false, disabled: true };
        let mut service = Service::<Toggle>::new(props);
        service.send(Event::Toggle);
        assert_eq!(*service.state(), State::Off);
    }

}
```
