# Adapter Component Dependency Inventory

This inventory records the first dependency-mapping pass for Leptos and Dioxus
adapter tasks. The machine-checkable source of truth is `spec/manifest.toml`.

| adapter | component | dependency | kind | blocking | source | reason |
| --- | --- | --- | --- | --- | --- | --- |
| leptos,dioxus | listbox | grid-list | boundary | false | `spec/manifest.toml` `[components.listbox]` | React Aria `layout:grid` belongs to GridList, not Listbox. |
| leptos,dioxus | select | grid-list | boundary | false | `spec/manifest.toml` `[components.select]` | Grid collection layouts belong to GridList; Select owns trigger and popup listbox selection. |
| leptos,dioxus | alert-dialog | dialog | requires | true | `spec/manifest.toml` `[components.alert-dialog]` | AlertDialog reuses the modal dialog interaction and semantics contract. |
| leptos,dioxus | alert-dialog | focus-scope | requires | true | `spec/manifest.toml` `[components.alert-dialog]` | AlertDialog focus trapping depends on FocusScope. |
| leptos,dioxus | dialog | focus-scope | requires | true | `spec/manifest.toml` `[components.dialog]` | Dialog focus trapping depends on FocusScope. |
| leptos,dioxus | drawer | dialog | composes | true | `spec/manifest.toml` `[components.drawer]` | Drawer is a dialog-like overlay surface and should reuse dialog focus and dismissal semantics. |
| leptos,dioxus | date-picker | calendar,date-field,button,field,form | composes | true | `spec/manifest.toml` `[components.date-picker]` | DatePicker composes calendar, segmented field, trigger, field, and form behavior. |
| leptos,dioxus | date-range-field | date-field,field,form | composes | true | `spec/manifest.toml` `[components.date-range-field]` | DateRangeField composes two DateField-style inputs plus field/form semantics. |
| leptos,dioxus | date-range-picker | date-field,range-calendar,button,field,form | composes | true | `spec/manifest.toml` `[components.date-range-picker]` | DateRangePicker composes child fields, range calendar, trigger, field, and form behavior. |
| leptos,dioxus | date-time-picker | date-field,time-field,calendar,button,field,form | composes | true | `spec/manifest.toml` `[components.date-time-picker]` | DateTimePicker composes segmented date/time fields, calendar, trigger, field, and form behavior. |
| leptos,dioxus | color-picker | color-area,color-slider,color-wheel,color-field,color-swatch-picker | composes | true | `spec/manifest.toml` `[components.color-picker]` | ColorPicker orchestrates the standalone color primitive adapters. |
| leptos,dioxus | contextual-help | popover | composes | true | `spec/manifest.toml` `[components.contextual-help]` | ContextualHelp displays content through a popover-style overlay. |
| leptos,dioxus | file-upload | drop-zone,file-trigger | composes | true | `spec/manifest.toml` `[components.file-upload]` | FileUpload reuses DropZone for drag/drop and FileTrigger for the native picker path. |

Subsystem-only dependencies remain in `foundation_deps`, `shared_deps`, and the
roadmap rather than `component_deps`. Examples: selection collection/typeahead
behavior depends on `ars-collections`, overlay layering depends on
`z-index-stacking`, date-time components depend on `ars-i18n` / `ars-forms`,
and Table/GridList/TagGroup/TreeView depend on collection primitives.
