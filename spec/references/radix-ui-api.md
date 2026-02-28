---
title: Radix UI Primitives API Reference
source: https://www.radix-ui.com/primitives/docs/components
repository: https://github.com/radix-ui/primitives
generated: 2026-03-27
total_components: 30 (28 stable + 2 preview)
---

Radix UI Primitives — Comprehensive API Reference

## Table of Contents

1. [Accordion](#1-accordion)
2. [AlertDialog](#2-alertdialog)
3. [AspectRatio](#3-aspectratio)
4. [Avatar](#4-avatar)
5. [Checkbox](#5-checkbox)
6. [Collapsible](#6-collapsible)
7. [ContextMenu](#7-contextmenu)
8. [Dialog](#8-dialog)
9. [DropdownMenu](#9-dropdownmenu)
10. [Form](#10-form)
11. [HoverCard](#11-hovercard)
12. [Label](#12-label)
13. [Menubar](#13-menubar)
14. [NavigationMenu](#14-navigationmenu)
15. [Popover](#15-popover)
16. [Progress](#16-progress)
17. [RadioGroup](#17-radiogroup)
18. [ScrollArea](#18-scrollarea)
19. [Select](#19-select)
20. [Separator](#20-separator)
21. [Slider](#21-slider)
22. [Switch](#22-switch)
23. [Tabs](#23-tabs)
24. [Toast](#24-toast)
25. [Toggle](#25-toggle)
26. [ToggleGroup](#26-togglegroup)
27. [Toolbar](#27-toolbar)
28. [Tooltip](#28-tooltip)
    A. [Portal (Utility)](#a-portal-utility)
    B. [VisuallyHidden (Utility)](#b-visuallyhidden-utility)
    C. [Slot (Utility)](#c-slot-utility)
    D. [OneTimePasswordField (Preview)](#d-onetimepasswordfield-preview)
    E. [PasswordToggleField (Preview)](#e-passwordtogglefield-preview)
    F. [Cross-Cutting Patterns](#f-cross-cutting-patterns)

---

## Complete Component Inventory

Radix UI Primitives provides **30 documented components** (28 stable + 2 preview):

### Stable Components (28)

**Input:** Checkbox, Switch, Slider, RadioGroup, Form
**Selection:** Select, DropdownMenu, ContextMenu, Menubar, ToggleGroup
**Overlay:** Dialog, AlertDialog, HoverCard, Popover, Tooltip, Toast
**Navigation:** Accordion, Tabs, NavigationMenu
**Data Display:** Avatar, Progress, AspectRatio
**Layout:** Collapsible, ScrollArea, Separator, Toolbar
**Utility:** Toggle, Label, Portal, VisuallyHidden, Slot (asChild infrastructure)

### Preview Components (2, unstable\_ prefix)

- OneTimePasswordField
- PasswordToggleField

### Components NOT in our spec list

- **Form** -- Radix has a form validation primitive
- **OneTimePasswordField** (preview) -- OTP input
- **PasswordToggleField** (preview) -- password visibility toggle

---

## 1. Accordion

**Radix name:** `accordion`
**Package:** `@radix-ui/react-accordion`

### 1.1 Anatomy (Parts)

- `Root` -- container
- `Item` -- collapsible section wrapper
- `Header` -- wraps trigger, heading level
- `Trigger` -- toggles item open/closed
- `Content` -- collapsible content

### 1.2 Root Props

| Prop          | Type                                                                        | Default    | Required | Description                                |
| ------------- | --------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------ |
| asChild       | boolean                                                                     | false      | No       | Merge props onto child element             |
| type          | `"single" \| "multiple"`                                                    | --         | Yes      | Whether one or multiple items can be open  |
| value         | string (single) / string[] (multiple)                                       | --         | No       | Controlled expanded item(s)                |
| defaultValue  | string (single) / string[] (multiple)                                       | -- / []    | No       | Default expanded item(s)                   |
| onValueChange | `(value: string) => void` (single) / `(value: string[]) => void` (multiple) | --         | No       | Called when expanded state changes         |
| collapsible   | boolean                                                                     | false      | No       | Allow closing all items when type="single" |
| disabled      | boolean                                                                     | false      | No       | Disable all items                          |
| dir           | `"ltr" \| "rtl"`                                                            | "ltr"      | No       | Reading direction                          |
| orientation   | `"horizontal" \| "vertical"`                                                | "vertical" | No       | Orientation                                |

### 1.3 Item Props

| Prop     | Type    | Default | Required | Description               |
| -------- | ------- | ------- | -------- | ------------------------- |
| asChild  | boolean | false   | No       | Merge props onto child    |
| disabled | boolean | false   | No       | Disable this item         |
| value    | string  | --      | Yes      | Unique value for the item |

### 1.4 Header Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 1.5 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 1.6 Content Props

| Prop       | Type    | Default | Description                       |
| ---------- | ------- | ------- | --------------------------------- |
| asChild    | boolean | false   | Merge props onto child            |
| forceMount | boolean | --      | Force mount for animation control |

### 1.7 Data Attributes

| Part    | Attribute          | Values                   |
| ------- | ------------------ | ------------------------ |
| Root    | [data-orientation] | "vertical", "horizontal" |
| Item    | [data-state]       | "open", "closed"         |
| Item    | [data-disabled]    | Present when disabled    |
| Item    | [data-orientation] | "vertical", "horizontal" |
| Header  | [data-state]       | "open", "closed"         |
| Header  | [data-disabled]    | Present when disabled    |
| Header  | [data-orientation] | "vertical", "horizontal" |
| Trigger | [data-state]       | "open", "closed"         |
| Trigger | [data-disabled]    | Present when disabled    |
| Trigger | [data-orientation] | "vertical", "horizontal" |
| Content | [data-state]       | "open", "closed"         |
| Content | [data-disabled]    | Present when disabled    |
| Content | [data-orientation] | "vertical", "horizontal" |

### 1.8 CSS Custom Properties

| Property                         | Description                            |
| -------------------------------- | -------------------------------------- |
| --radix-accordion-content-width  | Width of content when opening/closing  |
| --radix-accordion-content-height | Height of content when opening/closing |

---

## 2. AlertDialog

**Radix name:** `alert-dialog`
**Package:** `@radix-ui/react-alert-dialog`

### 2.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- button that opens dialog
- `Portal` -- portals overlay and content
- `Overlay` -- covers inert area
- `Content` -- dialog content
- `Title` -- accessible title
- `Description` -- accessible description
- `Cancel` -- cancel/close button
- `Action` -- confirm/action button

### 2.2 Root Props

| Prop         | Type                      | Default | Description                    |
| ------------ | ------------------------- | ------- | ------------------------------ |
| defaultOpen  | boolean                   | --      | Initial open state             |
| open         | boolean                   | --      | Controlled open state          |
| onOpenChange | `(open: boolean) => void` | --      | Called when open state changes |

### 2.3 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 2.4 Portal Props

| Prop       | Type        | Default       | Description                                   |
| ---------- | ----------- | ------------- | --------------------------------------------- |
| forceMount | boolean     | --            | Force mount; inherited by Overlay and Content |
| container  | HTMLElement | document.body | Portal target container                       |

### 2.5 Overlay Props

| Prop       | Type    | Default | Description                       |
| ---------- | ------- | ------- | --------------------------------- |
| asChild    | boolean | false   | Merge props onto child            |
| forceMount | boolean | --      | Force mount; inherits from Portal |

### 2.6 Content Props

| Prop             | Type                             | Default | Description                              |
| ---------------- | -------------------------------- | ------- | ---------------------------------------- |
| asChild          | boolean                          | false   | Merge props onto child                   |
| forceMount       | boolean                          | --      | Force mount; inherits from Portal        |
| onOpenAutoFocus  | `(event: Event) => void`         | --      | Focus moves into component after opening |
| onCloseAutoFocus | `(event: Event) => void`         | --      | Focus moves to trigger after closing     |
| onEscapeKeyDown  | `(event: KeyboardEvent) => void` | --      | Escape key pressed                       |

### 2.7 Cancel Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 2.8 Action Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 2.9 Title Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 2.10 Description Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 2.11 Data Attributes

| Part    | Attribute    | Values           |
| ------- | ------------ | ---------------- |
| Trigger | [data-state] | "open", "closed" |
| Overlay | [data-state] | "open", "closed" |
| Content | [data-state] | "open", "closed" |

---

## 3. AspectRatio

**Radix name:** `aspect-ratio`
**Package:** `@radix-ui/react-aspect-ratio`

### 3.1 Anatomy (Parts)

- `Root` -- container

### 3.2 Root Props

| Prop    | Type    | Default | Description              |
| ------- | ------- | ------- | ------------------------ |
| asChild | boolean | false   | Merge props onto child   |
| ratio   | number  | 1       | The desired aspect ratio |

### 3.3 Data Attributes

None.

### 3.4 CSS Custom Properties

None.

---

## 4. Avatar

**Radix name:** `avatar`
**Package:** `@radix-ui/react-avatar`

### 4.1 Anatomy (Parts)

- `Root` -- container
- `Image` -- the image element
- `Fallback` -- fallback when image unavailable

### 4.2 Root Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 4.3 Image Props

| Prop                  | Type                                                           | Default | Description                   |
| --------------------- | -------------------------------------------------------------- | ------- | ----------------------------- |
| asChild               | boolean                                                        | false   | Merge props onto child        |
| onLoadingStatusChange | `(status: "idle" \| "loading" \| "loaded" \| "error") => void` | --      | Image loading status callback |

### 4.4 Fallback Props

| Prop    | Type    | Default | Description                     |
| ------- | ------- | ------- | ------------------------------- |
| asChild | boolean | false   | Merge props onto child          |
| delayMs | number  | --      | Delay before rendering fallback |

### 4.5 Data Attributes

None.

### 4.6 CSS Custom Properties

None.

---

## 5. Checkbox

**Radix name:** `checkbox`
**Package:** `@radix-ui/react-checkbox`

### 5.1 Anatomy (Parts)

- `Root` -- the checkbox control (renders hidden input in forms)
- `Indicator` -- renders when checked/indeterminate

### 5.2 Root Props

| Prop            | Type                                            | Default | Description                  |
| --------------- | ----------------------------------------------- | ------- | ---------------------------- |
| asChild         | boolean                                         | false   | Merge props onto child       |
| defaultChecked  | `boolean \| 'indeterminate'`                    | --      | Initial checked state        |
| checked         | `boolean \| 'indeterminate'`                    | --      | Controlled checked state     |
| onCheckedChange | `(checked: boolean \| 'indeterminate') => void` | --      | Called when checked changes  |
| disabled        | boolean                                         | --      | Disable interaction          |
| required        | boolean                                         | --      | Required for form submission |
| name            | string                                          | --      | Form field name              |
| value           | string                                          | "on"    | Form field value             |

### 5.3 Indicator Props

| Prop       | Type    | Default | Description               |
| ---------- | ------- | ------- | ------------------------- |
| asChild    | boolean | false   | Merge props onto child    |
| forceMount | boolean | --      | Force mount for animation |

### 5.4 Data Attributes

| Part      | Attribute       | Values                                  |
| --------- | --------------- | --------------------------------------- |
| Root      | [data-state]    | "checked", "unchecked", "indeterminate" |
| Root      | [data-disabled] | Present when disabled                   |
| Indicator | [data-state]    | "checked", "unchecked", "indeterminate" |
| Indicator | [data-disabled] | Present when disabled                   |

---

## 6. Collapsible

**Radix name:** `collapsible`
**Package:** `@radix-ui/react-collapsible`

### 6.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- toggle button
- `Content` -- collapsible content

### 6.2 Root Props

| Prop         | Type                      | Default | Description              |
| ------------ | ------------------------- | ------- | ------------------------ |
| asChild      | boolean                   | false   | Merge props onto child   |
| defaultOpen  | boolean                   | --      | Initial open state       |
| open         | boolean                   | --      | Controlled open state    |
| onOpenChange | `(open: boolean) => void` | --      | Called when open changes |
| disabled     | boolean                   | --      | Disable interaction      |

### 6.3 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 6.4 Content Props

| Prop       | Type    | Default | Description               |
| ---------- | ------- | ------- | ------------------------- |
| asChild    | boolean | false   | Merge props onto child    |
| forceMount | boolean | --      | Force mount for animation |

### 6.5 Data Attributes

| Part    | Attribute       | Values                |
| ------- | --------------- | --------------------- |
| Root    | [data-state]    | "open", "closed"      |
| Root    | [data-disabled] | Present when disabled |
| Trigger | [data-state]    | "open", "closed"      |
| Trigger | [data-disabled] | Present when disabled |
| Content | [data-state]    | "open", "closed"      |
| Content | [data-disabled] | Present when disabled |

### 6.6 CSS Custom Properties

| Property                           | Description                            |
| ---------------------------------- | -------------------------------------- |
| --radix-collapsible-content-width  | Width of content when opening/closing  |
| --radix-collapsible-content-height | Height of content when opening/closing |

---

## 7. ContextMenu

**Radix name:** `context-menu`
**Package:** `@radix-ui/react-context-menu`

### 7.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- right-click/long-press area
- `Portal` -- portals content
- `Content` -- popup content
- `Arrow` -- optional arrow
- `Item` -- menu item
- `Group` -- item group
- `Label` -- non-focusable label
- `CheckboxItem` -- checkable item
- `RadioGroup` -- radio group container
- `RadioItem` -- radio item
- `ItemIndicator` -- checked/selected indicator
- `Separator` -- visual separator
- `Sub` -- submenu container
- `SubTrigger` -- opens submenu
- `SubContent` -- submenu content

### 7.2 Root Props

| Prop         | Type                      | Default | Description                    |
| ------------ | ------------------------- | ------- | ------------------------------ |
| dir          | `"ltr" \| "rtl"`          | --      | Reading direction              |
| onOpenChange | `(open: boolean) => void` | --      | Called when open state changes |
| modal        | boolean                   | true    | Modal mode                     |

### 7.3 Trigger Props

| Prop     | Type    | Default | Description                            |
| -------- | ------- | ------- | -------------------------------------- |
| asChild  | boolean | false   | Merge props onto child                 |
| disabled | boolean | false   | Disable (restores native context menu) |

### 7.4 Portal Props

| Prop       | Type        | Default       | Description   |
| ---------- | ----------- | ------------- | ------------- |
| forceMount | boolean     | --            | Force mount   |
| container  | HTMLElement | document.body | Portal target |

### 7.5 Content Props

| Prop                 | Type                                                            | Default   | Description                   |
| -------------------- | --------------------------------------------------------------- | --------- | ----------------------------- |
| asChild              | boolean                                                         | false     | Merge props onto child        |
| loop                 | boolean                                                         | false     | Loop keyboard navigation      |
| onCloseAutoFocus     | `(event: Event) => void`                                        | --        | Focus after closing           |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`                                | --        | Escape key                    |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`                      | --        | Pointer outside               |
| onFocusOutside       | `(event: FocusOutsideEvent) => void`                            | --        | Focus outside                 |
| onInteractOutside    | `(event: PointerDownOutsideEvent \| FocusOutsideEvent) => void` | --        | Any interaction outside       |
| forceMount           | boolean                                                         | --        | Force mount                   |
| alignOffset          | number                                                          | 0         | Vertical distance from anchor |
| avoidCollisions      | boolean                                                         | true      | Prevent boundary collisions   |
| collisionBoundary    | `Element \| null \| Array<Element \| null>`                     | []        | Collision boundary            |
| collisionPadding     | `number \| Partial<Record<Side, number>>`                       | 0         | Collision detection padding   |
| sticky               | `"partial" \| "always"`                                         | "partial" | Sticky behavior               |
| hideWhenDetached     | boolean                                                         | false     | Hide when trigger occluded    |

### 7.6 Arrow Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| width   | number  | 10      | Arrow width in pixels  |
| height  | number  | 5       | Arrow height in pixels |

### 7.7 Item Props

| Prop      | Type                     | Default | Description                                 |
| --------- | ------------------------ | ------- | ------------------------------------------- |
| asChild   | boolean                  | false   | Merge props onto child                      |
| disabled  | boolean                  | --      | Disable item                                |
| onSelect  | `(event: Event) => void` | --      | Item selected (preventDefault to keep open) |
| textValue | string                   | --      | Typeahead text override                     |

### 7.8 Group Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 7.9 Label Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 7.10 CheckboxItem Props

| Prop            | Type                         | Default | Description              |
| --------------- | ---------------------------- | ------- | ------------------------ |
| asChild         | boolean                      | false   | Merge props onto child   |
| checked         | `boolean \| 'indeterminate'` | --      | Controlled checked state |
| onCheckedChange | `(checked: boolean) => void` | --      | Checked state changed    |
| disabled        | boolean                      | --      | Disable item             |
| onSelect        | `(event: Event) => void`     | --      | Item selected            |
| textValue       | string                       | --      | Typeahead text override  |

### 7.11 RadioGroup Props

| Prop          | Type                      | Default | Description            |
| ------------- | ------------------------- | ------- | ---------------------- |
| asChild       | boolean                   | false   | Merge props onto child |
| value         | string                    | --      | Selected item value    |
| onValueChange | `(value: string) => void` | --      | Value changed          |

### 7.12 RadioItem Props

| Prop      | Type                     | Default | Description                  |
| --------- | ------------------------ | ------- | ---------------------------- |
| asChild   | boolean                  | false   | Merge props onto child       |
| value     | string                   | --      | Unique item value (required) |
| disabled  | boolean                  | --      | Disable item                 |
| onSelect  | `(event: Event) => void` | --      | Item selected                |
| textValue | string                   | --      | Typeahead text override      |

### 7.13 ItemIndicator Props

| Prop       | Type    | Default | Description            |
| ---------- | ------- | ------- | ---------------------- |
| asChild    | boolean | false   | Merge props onto child |
| forceMount | boolean | --      | Force mount            |

### 7.14 Separator Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 7.15 Sub Props

| Prop         | Type                      | Default | Description           |
| ------------ | ------------------------- | ------- | --------------------- |
| defaultOpen  | boolean                   | --      | Initial open state    |
| open         | boolean                   | --      | Controlled open state |
| onOpenChange | `(open: boolean) => void` | --      | Open state changed    |

### 7.16 SubTrigger Props

| Prop      | Type    | Default | Description             |
| --------- | ------- | ------- | ----------------------- |
| asChild   | boolean | false   | Merge props onto child  |
| disabled  | boolean | --      | Disable                 |
| textValue | string  | --      | Typeahead text override |

### 7.17 SubContent Props

| Prop                 | Type                                                            | Default   | Description              |
| -------------------- | --------------------------------------------------------------- | --------- | ------------------------ |
| asChild              | boolean                                                         | false     | Merge props onto child   |
| loop                 | boolean                                                         | false     | Loop keyboard navigation |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`                                | --        | Escape key               |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`                      | --        | Pointer outside          |
| onFocusOutside       | `(event: FocusOutsideEvent) => void`                            | --        | Focus outside            |
| onInteractOutside    | `(event: PointerDownOutsideEvent \| FocusOutsideEvent) => void` | --        | Any interaction outside  |
| forceMount           | boolean                                                         | --        | Force mount              |
| sideOffset           | number                                                          | 0         | Distance from trigger    |
| alignOffset          | number                                                          | 0         | Offset from alignment    |
| avoidCollisions      | boolean                                                         | true      | Prevent collisions       |
| collisionBoundary    | `Element \| null \| Array<Element \| null>`                     | []        | Collision boundary       |
| collisionPadding     | `number \| Partial<Record<Side, number>>`                       | 0         | Collision padding        |
| arrowPadding         | number                                                          | 0         | Arrow padding            |
| sticky               | `"partial" \| "always"`                                         | "partial" | Sticky behavior          |
| hideWhenDetached     | boolean                                                         | false     | Hide when occluded       |

### 7.18 Data Attributes

| Part          | Attribute          | Values                                  |
| ------------- | ------------------ | --------------------------------------- |
| Trigger       | [data-state]       | "open", "closed"                        |
| Content       | [data-state]       | "open", "closed"                        |
| Content       | [data-side]        | "left", "right", "bottom", "top"        |
| Content       | [data-align]       | "start", "end", "center"                |
| Item          | [data-highlighted] | Present when highlighted                |
| Item          | [data-disabled]    | Present when disabled                   |
| CheckboxItem  | [data-state]       | "checked", "unchecked", "indeterminate" |
| CheckboxItem  | [data-highlighted] | Present when highlighted                |
| CheckboxItem  | [data-disabled]    | Present when disabled                   |
| RadioItem     | [data-state]       | "checked", "unchecked", "indeterminate" |
| RadioItem     | [data-highlighted] | Present when highlighted                |
| RadioItem     | [data-disabled]    | Present when disabled                   |
| ItemIndicator | [data-state]       | "checked", "unchecked", "indeterminate" |
| SubTrigger    | [data-state]       | "open", "closed"                        |
| SubTrigger    | [data-highlighted] | Present when highlighted                |
| SubTrigger    | [data-disabled]    | Present when disabled                   |

### 7.19 CSS Custom Properties

| Property                                      | Description                                   |
| --------------------------------------------- | --------------------------------------------- |
| --radix-context-menu-content-transform-origin | Transform origin from content/arrow positions |
| --radix-context-menu-content-available-width  | Remaining width between trigger and boundary  |
| --radix-context-menu-content-available-height | Remaining height between trigger and boundary |
| --radix-context-menu-trigger-width            | Width of the trigger                          |
| --radix-context-menu-trigger-height           | Height of the trigger                         |

---

## 8. Dialog

**Radix name:** `dialog`
**Package:** `@radix-ui/react-dialog`

### 8.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- button that opens dialog
- `Portal` -- portals overlay and content
- `Overlay` -- covers inert area
- `Content` -- dialog content
- `Title` -- accessible title
- `Description` -- accessible description
- `Close` -- close button

### 8.2 Root Props

| Prop         | Type                      | Default | Description              |
| ------------ | ------------------------- | ------- | ------------------------ |
| defaultOpen  | boolean                   | --      | Initial open state       |
| open         | boolean                   | --      | Controlled open state    |
| onOpenChange | `(open: boolean) => void` | --      | Called when open changes |
| modal        | boolean                   | true    | Modal mode               |

### 8.3 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 8.4 Portal Props

| Prop       | Type        | Default       | Description                                   |
| ---------- | ----------- | ------------- | --------------------------------------------- |
| forceMount | boolean     | --            | Force mount; inherited by Overlay and Content |
| container  | HTMLElement | document.body | Portal target                                 |

### 8.5 Overlay Props

| Prop       | Type    | Default | Description                       |
| ---------- | ------- | ------- | --------------------------------- |
| asChild    | boolean | false   | Merge props onto child            |
| forceMount | boolean | --      | Force mount; inherits from Portal |

### 8.6 Content Props

| Prop                 | Type                                                            | Default | Description                        |
| -------------------- | --------------------------------------------------------------- | ------- | ---------------------------------- |
| asChild              | boolean                                                         | false   | Merge props onto child             |
| forceMount           | boolean                                                         | --      | Force mount; inherits from Portal  |
| onOpenAutoFocus      | `(event: Event) => void`                                        | --      | Focus into component after opening |
| onCloseAutoFocus     | `(event: Event) => void`                                        | --      | Focus to trigger after closing     |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`                                | --      | Escape key pressed                 |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`                      | --      | Pointer outside bounds             |
| onInteractOutside    | `(event: React.FocusEvent \| MouseEvent \| TouchEvent) => void` | --      | Any interaction outside            |

### 8.7 Close Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 8.8 Title Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 8.9 Description Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 8.10 Data Attributes

| Part    | Attribute    | Values           |
| ------- | ------------ | ---------------- |
| Trigger | [data-state] | "open", "closed" |
| Overlay | [data-state] | "open", "closed" |
| Content | [data-state] | "open", "closed" |

---

## 9. DropdownMenu

**Radix name:** `dropdown-menu`
**Package:** `@radix-ui/react-dropdown-menu`

### 9.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- button that toggles menu
- `Portal` -- portals content
- `Content` -- popup content
- `Arrow` -- optional arrow
- `Item` -- menu item
- `Group` -- item group
- `Label` -- non-focusable label
- `CheckboxItem` -- checkable item
- `RadioGroup` -- radio group container
- `RadioItem` -- radio item
- `ItemIndicator` -- checked/selected indicator
- `Separator` -- visual separator
- `Sub` -- submenu container
- `SubTrigger` -- opens submenu
- `SubContent` -- submenu content

### 9.2 Root Props

| Prop         | Type                      | Default | Description              |
| ------------ | ------------------------- | ------- | ------------------------ |
| defaultOpen  | boolean                   | --      | Initial open state       |
| open         | boolean                   | --      | Controlled open state    |
| onOpenChange | `(open: boolean) => void` | --      | Called when open changes |
| modal        | boolean                   | true    | Modal mode               |
| dir          | `"ltr" \| "rtl"`          | --      | Reading direction        |

### 9.3 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 9.4 Portal Props

| Prop       | Type        | Default       | Description   |
| ---------- | ----------- | ------------- | ------------- |
| forceMount | boolean     | --            | Force mount   |
| container  | HTMLElement | document.body | Portal target |

### 9.5 Content Props

| Prop                 | Type                                                            | Default   | Description              |
| -------------------- | --------------------------------------------------------------- | --------- | ------------------------ |
| asChild              | boolean                                                         | false     | Merge props onto child   |
| loop                 | boolean                                                         | false     | Loop keyboard navigation |
| onCloseAutoFocus     | `(event: Event) => void`                                        | --        | Focus after closing      |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`                                | --        | Escape key               |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`                      | --        | Pointer outside          |
| onFocusOutside       | `(event: FocusOutsideEvent) => void`                            | --        | Focus outside            |
| onInteractOutside    | `(event: PointerDownOutsideEvent \| FocusOutsideEvent) => void` | --        | Any interaction outside  |
| forceMount           | boolean                                                         | --        | Force mount              |
| side                 | `"top" \| "right" \| "bottom" \| "left"`                        | "bottom"  | Preferred side           |
| sideOffset           | number                                                          | 0         | Distance from trigger    |
| align                | `"start" \| "center" \| "end"`                                  | "center"  | Alignment                |
| alignOffset          | number                                                          | 0         | Alignment offset         |
| avoidCollisions      | boolean                                                         | true      | Prevent collisions       |
| collisionBoundary    | `Element \| null \| Array<Element \| null>`                     | []        | Collision boundary       |
| collisionPadding     | `number \| Partial<Record<Side, number>>`                       | 0         | Collision padding        |
| arrowPadding         | number                                                          | 0         | Arrow padding            |
| sticky               | `"partial" \| "always"`                                         | "partial" | Sticky behavior          |
| hideWhenDetached     | boolean                                                         | false     | Hide when occluded       |

### 9.6 Arrow Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| width   | number  | 10      | Arrow width px         |
| height  | number  | 5       | Arrow height px        |

### 9.7 Item Props

| Prop      | Type                     | Default | Description            |
| --------- | ------------------------ | ------- | ---------------------- |
| asChild   | boolean                  | false   | Merge props onto child |
| disabled  | boolean                  | --      | Disable item           |
| onSelect  | `(event: Event) => void` | --      | Item selected          |
| textValue | string                   | --      | Typeahead text         |

### 9.8 Group, Label, CheckboxItem, RadioGroup, RadioItem, ItemIndicator, Separator, Sub, SubTrigger, SubContent

Same props as ContextMenu equivalents (see ContextMenu section above).

### 9.9 Data Attributes

| Part          | Attribute          | Values                                  |
| ------------- | ------------------ | --------------------------------------- |
| Trigger       | [data-state]       | "open", "closed"                        |
| Trigger       | [data-disabled]    | Present when disabled                   |
| Content       | [data-state]       | "open", "closed"                        |
| Content       | [data-side]        | "left", "right", "bottom", "top"        |
| Content       | [data-align]       | "start", "end", "center"                |
| Content       | [data-orientation] | "vertical", "horizontal"                |
| Item          | [data-orientation] | "vertical", "horizontal"                |
| Item          | [data-highlighted] | Present when highlighted                |
| Item          | [data-disabled]    | Present when disabled                   |
| CheckboxItem  | [data-state]       | "checked", "unchecked", "indeterminate" |
| RadioItem     | [data-state]       | "checked", "unchecked", "indeterminate" |
| ItemIndicator | [data-state]       | "checked", "unchecked", "indeterminate" |
| SubTrigger    | [data-state]       | "open", "closed"                        |

### 9.10 CSS Custom Properties

| Property                                       | Description      |
| ---------------------------------------------- | ---------------- |
| --radix-dropdown-menu-content-transform-origin | Transform origin |
| --radix-dropdown-menu-content-available-width  | Available width  |
| --radix-dropdown-menu-content-available-height | Available height |
| --radix-dropdown-menu-trigger-width            | Trigger width    |
| --radix-dropdown-menu-trigger-height           | Trigger height   |

---

## 10. Form

**Radix name:** `form`
**Package:** `@radix-ui/react-form`

### 10.1 Anatomy (Parts)

- `Root` -- form container
- `Field` -- field wrapper (handles id/name/label)
- `Label` -- label element
- `Control` -- input element (default: input)
- `Message` -- validation message
- `ValidityState` -- render prop for validity state
- `Submit` -- submit button

### 10.2 Root Props

| Prop                | Type         | Default | Description                                             |
| ------------------- | ------------ | ------- | ------------------------------------------------------- |
| asChild             | boolean      | false   | Merge props onto child                                  |
| onClearServerErrors | `() => void` | --      | Called when form submitted/reset to clear server errors |

### 10.3 Field Props

| Prop          | Type    | Default | Description                 |
| ------------- | ------- | ------- | --------------------------- |
| asChild       | boolean | false   | Merge props onto child      |
| name          | string  | --      | Field name (required)       |
| serverInvalid | boolean | --      | Mark as server-side invalid |

### 10.4 Label Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 10.5 Control Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 10.6 Message Props

| Prop       | Type                | Default | Description                         |
| ---------- | ------------------- | ------- | ----------------------------------- |
| asChild    | boolean             | false   | Merge props onto child              |
| match      | Matcher (see below) | --      | Condition for visibility            |
| forceMatch | boolean             | false   | Force show (server-side validation) |
| name       | string              | --      | Target field (when outside Field)   |

Match type: `'badInput' | 'patternMismatch' | 'rangeOverflow' | 'rangeUnderflow' | 'stepMismatch' | 'tooLong' | 'tooShort' | 'typeMismatch' | 'valid' | 'valueMissing' | ((value: string, formData: FormData) => boolean) | ((value: string, formData: FormData) => Promise<boolean>)`

### 10.7 ValidityState Props

| Prop     | Type                                                  | Default | Description                       |
| -------- | ----------------------------------------------------- | ------- | --------------------------------- |
| children | `(validity: ValidityState \| undefined) => ReactNode` | --      | Render function                   |
| name     | string                                                | --      | Target field (when outside Field) |

### 10.8 Submit Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 10.9 Data Attributes

| Part    | Attribute      | Values               |
| ------- | -------------- | -------------------- |
| Field   | [data-invalid] | Present when invalid |
| Field   | [data-valid]   | Present when valid   |
| Label   | [data-invalid] | Present when invalid |
| Label   | [data-valid]   | Present when valid   |
| Control | [data-invalid] | Present when invalid |
| Control | [data-valid]   | Present when valid   |

---

## 11. HoverCard

**Radix name:** `hover-card`
**Package:** `@radix-ui/react-hover-card`

### 11.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- hover target
- `Portal` -- portals content
- `Content` -- popup content
- `Arrow` -- optional arrow

### 11.2 Root Props

| Prop         | Type                      | Default | Description                     |
| ------------ | ------------------------- | ------- | ------------------------------- |
| defaultOpen  | boolean                   | --      | Initial open state              |
| open         | boolean                   | --      | Controlled open state           |
| onOpenChange | `(open: boolean) => void` | --      | Called when open changes        |
| openDelay    | number                    | 700     | Mouse enter to open delay (ms)  |
| closeDelay   | number                    | 300     | Mouse leave to close delay (ms) |

### 11.3 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 11.4 Portal Props

| Prop       | Type        | Default       | Description   |
| ---------- | ----------- | ------------- | ------------- |
| forceMount | boolean     | --            | Force mount   |
| container  | HTMLElement | document.body | Portal target |

### 11.5 Content Props

| Prop              | Type                                        | Default   | Description            |
| ----------------- | ------------------------------------------- | --------- | ---------------------- |
| asChild           | boolean                                     | false     | Merge props onto child |
| forceMount        | boolean                                     | --        | Force mount            |
| side              | `"top" \| "right" \| "bottom" \| "left"`    | "bottom"  | Preferred side         |
| sideOffset        | number                                      | 0         | Distance from trigger  |
| align             | `"start" \| "center" \| "end"`              | "center"  | Alignment              |
| alignOffset       | number                                      | 0         | Alignment offset       |
| avoidCollisions   | boolean                                     | true      | Prevent collisions     |
| collisionBoundary | `Element \| null \| Array<Element \| null>` | []        | Collision boundary     |
| collisionPadding  | `number \| Partial<Record<Side, number>>`   | 0         | Collision padding      |
| arrowPadding      | number                                      | 0         | Arrow padding          |
| sticky            | `"partial" \| "always"`                     | "partial" | Sticky behavior        |
| hideWhenDetached  | boolean                                     | false     | Hide when occluded     |

### 11.6 Arrow Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| width   | number  | 10      | Arrow width px         |
| height  | number  | 5       | Arrow height px        |

### 11.7 Data Attributes

| Part    | Attribute    | Values                           |
| ------- | ------------ | -------------------------------- |
| Trigger | [data-state] | "open", "closed"                 |
| Content | [data-state] | "open", "closed"                 |
| Content | [data-side]  | "left", "right", "bottom", "top" |
| Content | [data-align] | "start", "end", "center"         |

### 11.8 CSS Custom Properties

| Property                                    | Description      |
| ------------------------------------------- | ---------------- |
| --radix-hover-card-content-transform-origin | Transform origin |
| --radix-hover-card-content-available-width  | Available width  |
| --radix-hover-card-content-available-height | Available height |
| --radix-hover-card-trigger-width            | Trigger width    |
| --radix-hover-card-trigger-height           | Trigger height   |

---

## 12. Label

**Radix name:** `label`
**Package:** `@radix-ui/react-label`

### 12.1 Anatomy (Parts)

- `Root` -- label element

### 12.2 Root Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| htmlFor | string  | --      | Associated element id  |

---

## 13. Menubar

**Radix name:** `menubar`
**Package:** `@radix-ui/react-menubar`

### 13.1 Anatomy (Parts)

- `Root` -- menubar container
- `Menu` -- single menu
- `Trigger` -- menu trigger button
- `Portal` -- portals content
- `Content` -- menu content
- `Arrow` -- optional arrow
- `Item` -- menu item
- `Group` -- item group
- `Label` -- non-focusable label
- `CheckboxItem` -- checkable item
- `RadioGroup` -- radio group
- `RadioItem` -- radio item
- `ItemIndicator` -- checked indicator
- `Separator` -- visual separator
- `Sub` -- submenu container
- `SubTrigger` -- submenu trigger
- `SubContent` -- submenu content

### 13.2 Root Props

| Prop          | Type                      | Default | Description              |
| ------------- | ------------------------- | ------- | ------------------------ |
| asChild       | boolean                   | false   | Merge props onto child   |
| defaultValue  | string                    | --      | Initial open menu value  |
| value         | string                    | --      | Controlled open menu     |
| onValueChange | `(value: string) => void` | --      | Value changed            |
| dir           | `"ltr" \| "rtl"`          | --      | Reading direction        |
| loop          | boolean                   | false   | Loop keyboard navigation |

### 13.3 Menu Props

| Prop    | Type    | Default | Description                       |
| ------- | ------- | ------- | --------------------------------- |
| asChild | boolean | false   | Merge props onto child            |
| value   | string  | --      | Unique value for controlled state |

### 13.4 Trigger, Portal, Content, Arrow, Item, Group, Label, CheckboxItem, RadioGroup, RadioItem, ItemIndicator, Separator, Sub, SubTrigger, SubContent

Same props structure as DropdownMenu equivalents. Content has: side, sideOffset, align, alignOffset, avoidCollisions, collisionBoundary, collisionPadding, arrowPadding, sticky, hideWhenDetached plus event handlers.

### 13.5 Data Attributes

Same as DropdownMenu.

### 13.6 CSS Custom Properties

| Property                                 | Description      |
| ---------------------------------------- | ---------------- |
| --radix-menubar-content-transform-origin | Transform origin |
| --radix-menubar-content-available-width  | Available width  |
| --radix-menubar-content-available-height | Available height |
| --radix-menubar-trigger-width            | Trigger width    |
| --radix-menubar-trigger-height           | Trigger height   |

---

## 14. NavigationMenu

**Radix name:** `navigation-menu`
**Package:** `@radix-ui/react-navigation-menu`

### 14.1 Anatomy (Parts)

- `Root` -- container
- `Sub` -- submenu container (replaces Root for nesting)
- `List` -- contains top-level items
- `Item` -- menu item
- `Trigger` -- toggles content
- `Content` -- associated content
- `Link` -- navigational link
- `Indicator` -- active trigger indicator
- `Viewport` -- optional external content viewport

### 14.2 Root Props

| Prop              | Type                         | Default      | Description               |
| ----------------- | ---------------------------- | ------------ | ------------------------- |
| defaultValue      | string                       | --           | Initial active item       |
| value             | string                       | --           | Controlled active item    |
| onValueChange     | `(value: string) => void`    | --           | Value changed             |
| delayDuration     | number                       | 200          | Mouse enter to open delay |
| skipDelayDuration | number                       | 300          | Skip delay window         |
| dir               | `"ltr" \| "rtl"`             | --           | Reading direction         |
| orientation       | `"horizontal" \| "vertical"` | "horizontal" | Orientation               |

### 14.3 Sub Props

| Prop          | Type                         | Default      | Description            |
| ------------- | ---------------------------- | ------------ | ---------------------- |
| defaultValue  | string                       | --           | Initial active item    |
| value         | string                       | --           | Controlled active item |
| onValueChange | `(value: string) => void`    | --           | Value changed          |
| orientation   | `"horizontal" \| "vertical"` | "horizontal" | Orientation            |

### 14.4 List Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 14.5 Item Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| value   | string  | --      | Unique value           |

### 14.6 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 14.7 Content Props

| Prop                 | Type                                                            | Default | Description            |
| -------------------- | --------------------------------------------------------------- | ------- | ---------------------- |
| asChild              | boolean                                                         | false   | Merge props onto child |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`                                | --      | Escape key             |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`                      | --      | Pointer outside        |
| onFocusOutside       | `(event: FocusOutsideEvent) => void`                            | --      | Focus outside          |
| onInteractOutside    | `(event: React.FocusEvent \| MouseEvent \| TouchEvent) => void` | --      | Interaction outside    |
| forceMount           | boolean                                                         | --      | Force mount            |

### 14.8 Link Props

| Prop     | Type                     | Default | Description            |
| -------- | ------------------------ | ------- | ---------------------- |
| asChild  | boolean                  | false   | Merge props onto child |
| active   | boolean                  | false   | Currently active page  |
| onSelect | `(event: Event) => void` | --      | Link selected          |

### 14.9 Indicator Props

| Prop       | Type    | Default | Description            |
| ---------- | ------- | ------- | ---------------------- |
| asChild    | boolean | false   | Merge props onto child |
| forceMount | boolean | --      | Force mount            |

### 14.10 Viewport Props

| Prop       | Type    | Default | Description            |
| ---------- | ------- | ------- | ---------------------- |
| asChild    | boolean | false   | Merge props onto child |
| forceMount | boolean | --      | Force mount            |

### 14.11 Data Attributes

| Part      | Attribute          | Values                                         |
| --------- | ------------------ | ---------------------------------------------- |
| Root      | [data-orientation] | "vertical", "horizontal"                       |
| Sub       | [data-orientation] | "vertical", "horizontal"                       |
| List      | [data-orientation] | "vertical", "horizontal"                       |
| Trigger   | [data-state]       | "open", "closed"                               |
| Trigger   | [data-disabled]    | Present when disabled                          |
| Content   | [data-state]       | "open", "closed"                               |
| Content   | [data-motion]      | "to-start", "to-end", "from-start", "from-end" |
| Content   | [data-orientation] | "vertical", "horizontal"                       |
| Link      | [data-active]      | Present when active                            |
| Indicator | [data-state]       | "visible", "hidden"                            |
| Indicator | [data-orientation] | "vertical", "horizontal"                       |
| Viewport  | [data-state]       | "open", "closed"                               |
| Viewport  | [data-orientation] | "vertical", "horizontal"                       |

### 14.12 CSS Custom Properties

| Property                                | Description                           |
| --------------------------------------- | ------------------------------------- |
| --radix-navigation-menu-viewport-width  | Viewport width (from active content)  |
| --radix-navigation-menu-viewport-height | Viewport height (from active content) |

---

## 15. Popover

**Radix name:** `popover`
**Package:** `@radix-ui/react-popover`

### 15.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- toggle button
- `Anchor` -- optional positioning anchor
- `Portal` -- portals content
- `Content` -- popup content
- `Arrow` -- optional arrow
- `Close` -- close button

### 15.2 Root Props

| Prop         | Type                      | Default | Description              |
| ------------ | ------------------------- | ------- | ------------------------ |
| defaultOpen  | boolean                   | --      | Initial open state       |
| open         | boolean                   | --      | Controlled open state    |
| onOpenChange | `(open: boolean) => void` | --      | Called when open changes |
| modal        | boolean                   | false   | Modal mode               |

### 15.3 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 15.4 Anchor Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 15.5 Portal Props

| Prop       | Type        | Default       | Description   |
| ---------- | ----------- | ------------- | ------------- |
| forceMount | boolean     | --            | Force mount   |
| container  | HTMLElement | document.body | Portal target |

### 15.6 Content Props

| Prop                 | Type                                                            | Default   | Description                    |
| -------------------- | --------------------------------------------------------------- | --------- | ------------------------------ |
| asChild              | boolean                                                         | false     | Merge props onto child         |
| onOpenAutoFocus      | `(event: Event) => void`                                        | --        | Focus into after opening       |
| onCloseAutoFocus     | `(event: Event) => void`                                        | --        | Focus to trigger after closing |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`                                | --        | Escape key                     |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`                      | --        | Pointer outside                |
| onFocusOutside       | `(event: FocusOutsideEvent) => void`                            | --        | Focus outside                  |
| onInteractOutside    | `(event: PointerDownOutsideEvent \| FocusOutsideEvent) => void` | --        | Interaction outside            |
| forceMount           | boolean                                                         | --        | Force mount                    |
| side                 | `"top" \| "right" \| "bottom" \| "left"`                        | "bottom"  | Preferred side                 |
| sideOffset           | number                                                          | 0         | Distance from anchor           |
| align                | `"start" \| "center" \| "end"`                                  | "center"  | Alignment                      |
| alignOffset          | number                                                          | 0         | Alignment offset               |
| avoidCollisions      | boolean                                                         | true      | Prevent collisions             |
| collisionBoundary    | `Element \| null \| Array<Element \| null>`                     | []        | Collision boundary             |
| collisionPadding     | `number \| Partial<Record<Side, number>>`                       | 0         | Collision padding              |
| arrowPadding         | number                                                          | 0         | Arrow padding                  |
| sticky               | `"partial" \| "always"`                                         | "partial" | Sticky behavior                |
| hideWhenDetached     | boolean                                                         | false     | Hide when occluded             |

### 15.7 Arrow Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| width   | number  | 10      | Arrow width px         |
| height  | number  | 5       | Arrow height px        |

### 15.8 Close Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 15.9 Data Attributes

| Part    | Attribute    | Values                           |
| ------- | ------------ | -------------------------------- |
| Trigger | [data-state] | "open", "closed"                 |
| Content | [data-state] | "open", "closed"                 |
| Content | [data-side]  | "left", "right", "bottom", "top" |
| Content | [data-align] | "start", "end", "center"         |

### 15.10 CSS Custom Properties

| Property                                 | Description      |
| ---------------------------------------- | ---------------- |
| --radix-popover-content-transform-origin | Transform origin |
| --radix-popover-content-available-width  | Available width  |
| --radix-popover-content-available-height | Available height |
| --radix-popover-trigger-width            | Trigger width    |
| --radix-popover-trigger-height           | Trigger height   |

---

## 16. Progress

**Radix name:** `progress`
**Package:** `@radix-ui/react-progress`

### 16.1 Anatomy (Parts)

- `Root` -- container
- `Indicator` -- visual progress indicator

### 16.2 Root Props

| Prop          | Type                                     | Default | Description                      |
| ------------- | ---------------------------------------- | ------- | -------------------------------- |
| asChild       | boolean                                  | false   | Merge props onto child           |
| value         | `number \| null`                         | --      | Progress value                   |
| max           | number                                   | --      | Maximum value                    |
| getValueLabel | `(value: number, max: number) => string` | --      | Custom accessible label function |

### 16.3 Indicator Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 16.4 Data Attributes

| Part      | Attribute    | Values                                 |
| --------- | ------------ | -------------------------------------- |
| Root      | [data-state] | "complete", "indeterminate", "loading" |
| Root      | [data-value] | The current value                      |
| Root      | [data-max]   | The max value                          |
| Indicator | [data-state] | "complete", "indeterminate", "loading" |
| Indicator | [data-value] | The current value                      |
| Indicator | [data-max]   | The max value                          |

---

## 17. RadioGroup

**Radix name:** `radio-group`
**Package:** `@radix-ui/react-radio-group`

### 17.1 Anatomy (Parts)

- `Root` -- group container
- `Item` -- radio button
- `Indicator` -- checked indicator

### 17.2 Root Props

| Prop          | Type                                      | Default   | Description              |
| ------------- | ----------------------------------------- | --------- | ------------------------ |
| asChild       | boolean                                   | false     | Merge props onto child   |
| defaultValue  | string                                    | --        | Initial checked item     |
| value         | string                                    | --        | Controlled checked item  |
| onValueChange | `(value: string) => void`                 | --        | Value changed            |
| disabled      | boolean                                   | --        | Disable all items        |
| name          | string                                    | --        | Form field name          |
| required      | boolean                                   | --        | Required for form        |
| orientation   | `"horizontal" \| "vertical" \| undefined` | undefined | Orientation              |
| dir           | `"ltr" \| "rtl"`                          | --        | Reading direction        |
| loop          | boolean                                   | true      | Loop keyboard navigation |

### 17.3 Item Props

| Prop     | Type    | Default | Description            |
| -------- | ------- | ------- | ---------------------- |
| asChild  | boolean | false   | Merge props onto child |
| value    | string  | --      | Item value             |
| disabled | boolean | --      | Disable item           |
| required | boolean | --      | Required               |

### 17.4 Indicator Props

| Prop       | Type    | Default | Description            |
| ---------- | ------- | ------- | ---------------------- |
| asChild    | boolean | false   | Merge props onto child |
| forceMount | boolean | --      | Force mount            |

### 17.5 Data Attributes

| Part      | Attribute       | Values                 |
| --------- | --------------- | ---------------------- |
| Root      | [data-disabled] | Present when disabled  |
| Item      | [data-state]    | "checked", "unchecked" |
| Item      | [data-disabled] | Present when disabled  |
| Indicator | [data-state]    | "checked", "unchecked" |
| Indicator | [data-disabled] | Present when disabled  |

---

## 18. ScrollArea

**Radix name:** `scroll-area`
**Package:** `@radix-ui/react-scroll-area`

### 18.1 Anatomy (Parts)

- `Root` -- container
- `Viewport` -- scrollable area
- `Scrollbar` -- scrollbar (horizontal/vertical)
- `Thumb` -- scrollbar thumb
- `Corner` -- corner where scrollbars meet

### 18.2 Root Props

| Prop            | Type                                        | Default | Description                         |
| --------------- | ------------------------------------------- | ------- | ----------------------------------- |
| asChild         | boolean                                     | false   | Merge props onto child              |
| type            | `"auto" \| "always" \| "scroll" \| "hover"` | "hover" | Scrollbar visibility mode           |
| scrollHideDelay | number                                      | 600     | Delay before hiding scrollbars (ms) |
| dir             | `"ltr" \| "rtl"`                            | --      | Reading direction                   |
| nonce           | string                                      | --      | CSP nonce for inline styles         |

### 18.3 Viewport Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 18.4 Scrollbar Props

| Prop        | Type                         | Default    | Description            |
| ----------- | ---------------------------- | ---------- | ---------------------- |
| asChild     | boolean                      | false      | Merge props onto child |
| forceMount  | boolean                      | --         | Force mount            |
| orientation | `"horizontal" \| "vertical"` | "vertical" | Scrollbar orientation  |

### 18.5 Thumb Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 18.6 Corner Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 18.7 Data Attributes

| Part      | Attribute          | Values                   |
| --------- | ------------------ | ------------------------ |
| Scrollbar | [data-state]       | "visible", "hidden"      |
| Scrollbar | [data-orientation] | "vertical", "horizontal" |
| Thumb     | [data-state]       | "visible", "hidden"      |

---

## 19. Select

**Radix name:** `select`
**Package:** `@radix-ui/react-select`

### 19.1 Anatomy (Parts)

- `Root` -- container
- `Trigger` -- button that opens select
- `Value` -- displays selected value
- `Icon` -- dropdown icon
- `Portal` -- portals content
- `Content` -- dropdown content
- `Viewport` -- scrolling viewport
- `Item` -- selectable item
- `ItemText` -- item display text
- `ItemIndicator` -- selected indicator
- `ScrollUpButton` -- scroll up affordance
- `ScrollDownButton` -- scroll down affordance
- `Group` -- item group
- `Label` -- group label
- `Separator` -- visual separator
- `Arrow` -- optional arrow (popper mode only)

### 19.2 Root Props

| Prop          | Type                      | Default | Description            |
| ------------- | ------------------------- | ------- | ---------------------- |
| defaultValue  | string                    | --      | Initial selected value |
| value         | string                    | --      | Controlled value       |
| onValueChange | `(value: string) => void` | --      | Value changed          |
| defaultOpen   | boolean                   | --      | Initial open state     |
| open          | boolean                   | --      | Controlled open state  |
| onOpenChange  | `(open: boolean) => void` | --      | Open state changed     |
| dir           | `"ltr" \| "rtl"`          | --      | Reading direction      |
| name          | string                    | --      | Form field name        |
| disabled      | boolean                   | --      | Disable interaction    |
| required      | boolean                   | --      | Required for form      |

### 19.3 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 19.4 Value Props

| Prop        | Type      | Default | Description               |
| ----------- | --------- | ------- | ------------------------- |
| asChild     | boolean   | false   | Merge props onto child    |
| placeholder | ReactNode | --      | Placeholder when no value |

### 19.5 Icon Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 19.6 Portal Props

| Prop      | Type        | Default       | Description   |
| --------- | ----------- | ------------- | ------------- |
| container | HTMLElement | document.body | Portal target |

### 19.7 Content Props

| Prop                 | Type                                        | Default        | Description                    |
| -------------------- | ------------------------------------------- | -------------- | ------------------------------ |
| asChild              | boolean                                     | false          | Merge props onto child         |
| onCloseAutoFocus     | `(event: Event) => void`                    | --             | Focus after closing            |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`            | --             | Escape key                     |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`  | --             | Pointer outside                |
| position             | `"item-aligned" \| "popper"`                | "item-aligned" | Positioning mode               |
| side                 | `"top" \| "right" \| "bottom" \| "left"`    | "bottom"       | Side (popper only)             |
| sideOffset           | number                                      | 0              | Side offset (popper only)      |
| align                | `"start" \| "center" \| "end"`              | "start"        | Alignment (popper only)        |
| alignOffset          | number                                      | 0              | Alignment offset (popper only) |
| avoidCollisions      | boolean                                     | true           | Avoid collisions (popper only) |
| collisionBoundary    | `Element \| null \| Array<Element \| null>` | []             | Collision boundary (popper)    |
| collisionPadding     | `number \| Partial<Record<Side, number>>`   | 10             | Collision padding (popper)     |
| arrowPadding         | number                                      | 0              | Arrow padding (popper)         |
| sticky               | `"partial" \| "always"`                     | "partial"      | Sticky (popper)                |
| hideWhenDetached     | boolean                                     | false          | Hide when occluded (popper)    |

### 19.8 Viewport Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 19.9 Item Props

| Prop      | Type    | Default | Description            |
| --------- | ------- | ------- | ---------------------- |
| asChild   | boolean | false   | Merge props onto child |
| value     | string  | --      | Item value (required)  |
| disabled  | boolean | --      | Disable item           |
| textValue | string  | --      | Typeahead text         |

### 19.10 ItemText, ItemIndicator, ScrollUpButton, ScrollDownButton, Group, Label, Separator Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 19.11 Arrow Props (popper mode only)

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| width   | number  | 10      | Arrow width px         |
| height  | number  | 5       | Arrow height px        |

### 19.12 Data Attributes

| Part    | Attribute          | Values                           |
| ------- | ------------------ | -------------------------------- |
| Trigger | [data-state]       | "open", "closed"                 |
| Trigger | [data-disabled]    | Present when disabled            |
| Trigger | [data-placeholder] | Present when has placeholder     |
| Content | [data-state]       | "open", "closed"                 |
| Content | [data-side]        | "left", "right", "bottom", "top" |
| Content | [data-align]       | "start", "end", "center"         |
| Item    | [data-state]       | "checked", "unchecked"           |
| Item    | [data-highlighted] | Present when highlighted         |
| Item    | [data-disabled]    | Present when disabled            |

### 19.13 CSS Custom Properties (popper mode only)

| Property                                | Description      |
| --------------------------------------- | ---------------- |
| --radix-select-content-transform-origin | Transform origin |
| --radix-select-content-available-width  | Available width  |
| --radix-select-content-available-height | Available height |
| --radix-select-trigger-width            | Trigger width    |
| --radix-select-trigger-height           | Trigger height   |

---

## 20. Separator

**Radix name:** `separator`
**Package:** `@radix-ui/react-separator`

### 20.1 Anatomy (Parts)

- `Root` -- the separator

### 20.2 Root Props

| Prop        | Type                         | Default      | Description                      |
| ----------- | ---------------------------- | ------------ | -------------------------------- |
| asChild     | boolean                      | false        | Merge props onto child           |
| orientation | `"horizontal" \| "vertical"` | "horizontal" | Orientation                      |
| decorative  | boolean                      | --           | Purely visual (no semantic role) |

### 20.3 Data Attributes

| Part | Attribute          | Values                   |
| ---- | ------------------ | ------------------------ |
| Root | [data-orientation] | "vertical", "horizontal" |

---

## 21. Slider

**Radix name:** `slider`
**Package:** `@radix-ui/react-slider`

### 21.1 Anatomy (Parts)

- `Root` -- container (renders hidden inputs in forms)
- `Track` -- slider track
- `Range` -- filled range
- `Thumb` -- draggable thumb (multiple for range)

### 21.2 Root Props

| Prop                  | Type                         | Default      | Description                          |
| --------------------- | ---------------------------- | ------------ | ------------------------------------ |
| asChild               | boolean                      | false        | Merge props onto child               |
| defaultValue          | number[]                     | --           | Initial value(s)                     |
| value                 | number[]                     | --           | Controlled value(s)                  |
| onValueChange         | `(value: number[]) => void`  | --           | Value changed (during drag)          |
| onValueCommit         | `(value: number[]) => void`  | --           | Value committed (end of interaction) |
| name                  | string                       | --           | Form field name                      |
| disabled              | boolean                      | false        | Disable                              |
| orientation           | `"horizontal" \| "vertical"` | "horizontal" | Orientation                          |
| dir                   | `"ltr" \| "rtl"`             | --           | Reading direction                    |
| inverted              | boolean                      | false        | Visually inverted                    |
| min                   | number                       | 0            | Minimum value                        |
| max                   | number                       | 100          | Maximum value                        |
| step                  | number                       | 1            | Step interval                        |
| minStepsBetweenThumbs | number                       | 0            | Min steps between thumbs             |
| form                  | string                       | --           | Associated form id                   |

### 21.3 Track Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 21.4 Range Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 21.5 Thumb Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 21.6 Data Attributes

| Part  | Attribute          | Values                   |
| ----- | ------------------ | ------------------------ |
| Root  | [data-disabled]    | Present when disabled    |
| Root  | [data-orientation] | "vertical", "horizontal" |
| Track | [data-disabled]    | Present when disabled    |
| Track | [data-orientation] | "vertical", "horizontal" |
| Range | [data-disabled]    | Present when disabled    |
| Range | [data-orientation] | "vertical", "horizontal" |
| Thumb | [data-disabled]    | Present when disabled    |
| Thumb | [data-orientation] | "vertical", "horizontal" |

---

## 22. Switch

**Radix name:** `switch`
**Package:** `@radix-ui/react-switch`

### 22.1 Anatomy (Parts)

- `Root` -- the switch control (renders hidden input in forms)
- `Thumb` -- visual toggle indicator

### 22.2 Root Props

| Prop            | Type                         | Default | Description              |
| --------------- | ---------------------------- | ------- | ------------------------ |
| asChild         | boolean                      | false   | Merge props onto child   |
| defaultChecked  | boolean                      | --      | Initial checked state    |
| checked         | boolean                      | --      | Controlled checked state |
| onCheckedChange | `(checked: boolean) => void` | --      | Checked state changed    |
| disabled        | boolean                      | --      | Disable                  |
| required        | boolean                      | --      | Required for form        |
| name            | string                       | --      | Form field name          |
| value           | string                       | "on"    | Form field value         |

### 22.3 Thumb Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 22.4 Data Attributes

| Part  | Attribute       | Values                 |
| ----- | --------------- | ---------------------- |
| Root  | [data-state]    | "checked", "unchecked" |
| Root  | [data-disabled] | Present when disabled  |
| Thumb | [data-state]    | "checked", "unchecked" |
| Thumb | [data-disabled] | Present when disabled  |

---

## 23. Tabs

**Radix name:** `tabs`
**Package:** `@radix-ui/react-tabs`

### 23.1 Anatomy (Parts)

- `Root` -- container
- `List` -- trigger list
- `Trigger` -- tab button
- `Content` -- tab panel

### 23.2 Root Props

| Prop           | Type                                      | Default      | Description            |
| -------------- | ----------------------------------------- | ------------ | ---------------------- |
| asChild        | boolean                                   | false        | Merge props onto child |
| defaultValue   | string                                    | --           | Initial active tab     |
| value          | string                                    | --           | Controlled active tab  |
| onValueChange  | `(value: string) => void`                 | --           | Value changed          |
| orientation    | `"horizontal" \| "vertical" \| undefined` | "horizontal" | Orientation            |
| dir            | `"ltr" \| "rtl"`                          | --           | Reading direction      |
| activationMode | `"automatic" \| "manual"`                 | "automatic"  | Activation mode        |

### 23.3 List Props

| Prop    | Type    | Default | Description              |
| ------- | ------- | ------- | ------------------------ |
| asChild | boolean | false   | Merge props onto child   |
| loop    | boolean | true    | Loop keyboard navigation |

### 23.4 Trigger Props

| Prop     | Type    | Default | Description            |
| -------- | ------- | ------- | ---------------------- |
| asChild  | boolean | false   | Merge props onto child |
| value    | string  | --      | Tab value (required)   |
| disabled | boolean | false   | Disable tab            |

### 23.5 Content Props

| Prop       | Type    | Default | Description            |
| ---------- | ------- | ------- | ---------------------- |
| asChild    | boolean | false   | Merge props onto child |
| value      | string  | --      | Tab value (required)   |
| forceMount | boolean | --      | Force mount            |

### 23.6 Data Attributes

| Part    | Attribute          | Values                   |
| ------- | ------------------ | ------------------------ |
| Root    | [data-orientation] | "vertical", "horizontal" |
| List    | [data-orientation] | "vertical", "horizontal" |
| Trigger | [data-state]       | "active", "inactive"     |
| Trigger | [data-disabled]    | Present when disabled    |
| Trigger | [data-orientation] | "vertical", "horizontal" |
| Content | [data-state]       | "active", "inactive"     |
| Content | [data-orientation] | "vertical", "horizontal" |

---

## 24. Toast

**Radix name:** `toast`
**Package:** `@radix-ui/react-toast`

### 24.1 Anatomy (Parts)

- `Provider` -- wraps app for global toast config
- `Root` -- individual toast
- `Viewport` -- fixed toast area
- `Title` -- toast title
- `Description` -- toast message
- `Action` -- actionable button
- `Close` -- dismiss button

### 24.2 Provider Props

| Prop           | Type                                  | Default        | Description                    |
| -------------- | ------------------------------------- | -------------- | ------------------------------ |
| duration       | number                                | 5000           | Auto-close duration (ms)       |
| label          | string                                | "Notification" | Screen reader label (required) |
| swipeDirection | `"right" \| "left" \| "up" \| "down"` | "right"        | Swipe-to-close direction       |
| swipeThreshold | number                                | 50             | Swipe distance threshold (px)  |

### 24.3 Viewport Props

| Prop    | Type     | Default                    | Description                 |
| ------- | -------- | -------------------------- | --------------------------- |
| asChild | boolean  | false                      | Merge props onto child      |
| hotkey  | string[] | ["F8"]                     | Keyboard shortcut for focus |
| label   | string   | "Notifications ({hotkey})" | Screen reader label         |

### 24.4 Root Props

| Prop            | Type                             | Default      | Description                |
| --------------- | -------------------------------- | ------------ | -------------------------- |
| asChild         | boolean                          | false        | Merge props onto child     |
| type            | `"foreground" \| "background"`   | "foreground" | Accessibility sensitivity  |
| duration        | number                           | --           | Override provider duration |
| defaultOpen     | boolean                          | true         | Initial open state         |
| open            | boolean                          | --           | Controlled open state      |
| onOpenChange    | `(open: boolean) => void`        | --           | Open state changed         |
| onEscapeKeyDown | `(event: KeyboardEvent) => void` | --           | Escape key                 |
| onPause         | `() => void`                     | --           | Dismiss timer paused       |
| onResume        | `() => void`                     | --           | Dismiss timer resumed      |
| onSwipeStart    | `(event: SwipeEvent) => void`    | --           | Swipe started              |
| onSwipeMove     | `(event: SwipeEvent) => void`    | --           | Swipe in progress          |
| onSwipeEnd      | `(event: SwipeEvent) => void`    | --           | Swipe ended                |
| onSwipeCancel   | `(event: SwipeEvent) => void`    | --           | Swipe cancelled            |
| forceMount      | boolean                          | --           | Force mount                |

### 24.5 Title Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 24.6 Description Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 24.7 Action Props

| Prop    | Type    | Default | Description                                                  |
| ------- | ------- | ------- | ------------------------------------------------------------ |
| asChild | boolean | false   | Merge props onto child                                       |
| altText | string  | --      | Alternative action description for screen readers (required) |

### 24.8 Close Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 24.9 Data Attributes

| Part | Attribute              | Values                           |
| ---- | ---------------------- | -------------------------------- |
| Root | [data-state]           | "open", "closed"                 |
| Root | [data-swipe]           | "start", "move", "cancel", "end" |
| Root | [data-swipe-direction] | "up", "down", "left", "right"    |

### 24.10 CSS Custom Properties

| Property                   | Description                 |
| -------------------------- | --------------------------- |
| --radix-toast-swipe-move-x | Horizontal swipe offset     |
| --radix-toast-swipe-move-y | Vertical swipe offset       |
| --radix-toast-swipe-end-x  | Horizontal swipe end offset |
| --radix-toast-swipe-end-y  | Vertical swipe end offset   |

---

## 25. Toggle

**Radix name:** `toggle`
**Package:** `@radix-ui/react-toggle`

### 25.1 Anatomy (Parts)

- `Root` -- the toggle button

### 25.2 Root Props

| Prop            | Type                         | Default | Description              |
| --------------- | ---------------------------- | ------- | ------------------------ |
| asChild         | boolean                      | false   | Merge props onto child   |
| defaultPressed  | boolean                      | --      | Initial pressed state    |
| pressed         | boolean                      | --      | Controlled pressed state |
| onPressedChange | `(pressed: boolean) => void` | --      | Pressed state changed    |
| disabled        | boolean                      | --      | Disable                  |

### 25.3 Data Attributes

| Part | Attribute       | Values                |
| ---- | --------------- | --------------------- |
| Root | [data-state]    | "on", "off"           |
| Root | [data-disabled] | Present when disabled |

---

## 26. ToggleGroup

**Radix name:** `toggle-group`
**Package:** `@radix-ui/react-toggle-group`

### 26.1 Anatomy (Parts)

- `Root` -- group container
- `Item` -- toggle item

### 26.2 Root Props

| Prop          | Type                                                                        | Default   | Description                             |
| ------------- | --------------------------------------------------------------------------- | --------- | --------------------------------------- |
| asChild       | boolean                                                                     | false     | Merge props onto child                  |
| type          | `"single" \| "multiple"`                                                    | --        | Single or multiple selection (required) |
| value         | string (single) / string[] (multiple)                                       | -- / []   | Controlled value(s)                     |
| defaultValue  | string (single) / string[] (multiple)                                       | -- / []   | Default value(s)                        |
| onValueChange | `(value: string) => void` (single) / `(value: string[]) => void` (multiple) | --        | Value changed                           |
| disabled      | boolean                                                                     | false     | Disable all items                       |
| rovingFocus   | boolean                                                                     | true      | Enable roving tabindex                  |
| orientation   | `"horizontal" \| "vertical" \| undefined`                                   | undefined | Orientation                             |
| dir           | `"ltr" \| "rtl"`                                                            | --        | Reading direction                       |
| loop          | boolean                                                                     | true      | Loop keyboard navigation                |

### 26.3 Item Props

| Prop     | Type    | Default | Description             |
| -------- | ------- | ------- | ----------------------- |
| asChild  | boolean | false   | Merge props onto child  |
| value    | string  | --      | Unique value (required) |
| disabled | boolean | --      | Disable item            |

### 26.4 Data Attributes

| Part | Attribute          | Values                   |
| ---- | ------------------ | ------------------------ |
| Root | [data-orientation] | "vertical", "horizontal" |
| Item | [data-state]       | "on", "off"              |
| Item | [data-disabled]    | Present when disabled    |
| Item | [data-orientation] | "vertical", "horizontal" |

---

## 27. Toolbar

**Radix name:** `toolbar`
**Package:** `@radix-ui/react-toolbar`

### 27.1 Anatomy (Parts)

- `Root` -- toolbar container
- `Button` -- button item
- `Link` -- link item
- `ToggleGroup` -- embedded toggle group
- `ToggleItem` -- toggle group item
- `Separator` -- visual separator

### 27.2 Root Props

| Prop        | Type                                      | Default      | Description              |
| ----------- | ----------------------------------------- | ------------ | ------------------------ |
| asChild     | boolean                                   | false        | Merge props onto child   |
| orientation | `"horizontal" \| "vertical" \| undefined` | "horizontal" | Orientation              |
| dir         | `"ltr" \| "rtl"`                          | --           | Reading direction        |
| loop        | boolean                                   | true         | Loop keyboard navigation |

### 27.3 Button Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 27.4 Link Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 27.5 ToggleGroup Props

Same as standalone ToggleGroup.Root (type, value, defaultValue, onValueChange, disabled).

### 27.6 ToggleItem Props

| Prop     | Type    | Default | Description             |
| -------- | ------- | ------- | ----------------------- |
| asChild  | boolean | false   | Merge props onto child  |
| value    | string  | --      | Unique value (required) |
| disabled | boolean | --      | Disable item            |

### 27.7 Separator Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 27.8 Data Attributes

| Part        | Attribute          | Values                   |
| ----------- | ------------------ | ------------------------ |
| Root        | [data-orientation] | "vertical", "horizontal" |
| Button      | [data-orientation] | "vertical", "horizontal" |
| ToggleGroup | [data-orientation] | "vertical", "horizontal" |
| ToggleItem  | [data-state]       | "on", "off"              |
| ToggleItem  | [data-disabled]    | Present when disabled    |
| ToggleItem  | [data-orientation] | "vertical", "horizontal" |
| Separator   | [data-orientation] | "vertical", "horizontal" |

---

## 28. Tooltip

**Radix name:** `tooltip`
**Package:** `@radix-ui/react-tooltip`

### 28.1 Anatomy (Parts)

- `Provider` -- wraps app for global config
- `Root` -- individual tooltip
- `Trigger` -- hover/focus target
- `Portal` -- portals content
- `Content` -- tooltip content
- `Arrow` -- optional arrow

### 28.2 Provider Props

| Prop                    | Type    | Default | Description               |
| ----------------------- | ------- | ------- | ------------------------- |
| delayDuration           | number  | 700     | Open delay (ms)           |
| skipDelayDuration       | number  | 300     | Skip delay window (ms)    |
| disableHoverableContent | boolean | --      | Disable hoverable content |

### 28.3 Root Props

| Prop                    | Type                      | Default | Description               |
| ----------------------- | ------------------------- | ------- | ------------------------- |
| defaultOpen             | boolean                   | --      | Initial open state        |
| open                    | boolean                   | --      | Controlled open state     |
| onOpenChange            | `(open: boolean) => void` | --      | Open state changed        |
| delayDuration           | number                    | 700     | Override provider delay   |
| disableHoverableContent | boolean                   | false   | Disable hoverable content |

### 28.4 Trigger Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### 28.5 Portal Props

| Prop       | Type        | Default       | Description   |
| ---------- | ----------- | ------------- | ------------- |
| forceMount | boolean     | --            | Force mount   |
| container  | HTMLElement | document.body | Portal target |

### 28.6 Content Props

| Prop                 | Type                                        | Default   | Description                |
| -------------------- | ------------------------------------------- | --------- | -------------------------- |
| asChild              | boolean                                     | false     | Merge props onto child     |
| aria-label           | string                                      | --        | Custom screen reader label |
| onEscapeKeyDown      | `(event: KeyboardEvent) => void`            | --        | Escape key                 |
| onPointerDownOutside | `(event: PointerDownOutsideEvent) => void`  | --        | Pointer outside            |
| forceMount           | boolean                                     | --        | Force mount                |
| side                 | `"top" \| "right" \| "bottom" \| "left"`    | "top"     | Preferred side             |
| sideOffset           | number                                      | 0         | Distance from trigger      |
| align                | `"start" \| "center" \| "end"`              | "center"  | Alignment                  |
| alignOffset          | number                                      | 0         | Alignment offset           |
| avoidCollisions      | boolean                                     | true      | Prevent collisions         |
| collisionBoundary    | `Element \| null \| Array<Element \| null>` | []        | Collision boundary         |
| collisionPadding     | `number \| Partial<Record<Side, number>>`   | 0         | Collision padding          |
| arrowPadding         | number                                      | 0         | Arrow padding              |
| sticky               | `"partial" \| "always"`                     | "partial" | Sticky behavior            |
| hideWhenDetached     | boolean                                     | false     | Hide when occluded         |

### 28.7 Arrow Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |
| width   | number  | 10      | Arrow width px         |
| height  | number  | 5       | Arrow height px        |

### 28.8 Data Attributes

| Part    | Attribute    | Values                                   |
| ------- | ------------ | ---------------------------------------- |
| Trigger | [data-state] | "closed", "delayed-open", "instant-open" |
| Content | [data-state] | "closed", "delayed-open", "instant-open" |
| Content | [data-side]  | "left", "right", "bottom", "top"         |
| Content | [data-align] | "start", "end", "center"                 |

### 28.9 CSS Custom Properties

| Property                                 | Description      |
| ---------------------------------------- | ---------------- |
| --radix-tooltip-content-transform-origin | Transform origin |
| --radix-tooltip-content-available-width  | Available width  |
| --radix-tooltip-content-available-height | Available height |
| --radix-tooltip-trigger-width            | Trigger width    |
| --radix-tooltip-trigger-height           | Trigger height   |

---

## A. Portal (Utility)

**Radix name:** `portal`
**Package:** `@radix-ui/react-portal`

### A.1 Anatomy (Parts)

- `Root` -- portal container

### A.2 Root Props

| Prop      | Type        | Default       | Description            |
| --------- | ----------- | ------------- | ---------------------- |
| asChild   | boolean     | false         | Merge props onto child |
| container | HTMLElement | document.body | Portal target          |

---

## B. VisuallyHidden (Utility)

**Radix name:** `visually-hidden`
**Package:** `@radix-ui/react-visually-hidden`

### B.1 Anatomy (Parts)

- `Root` -- visually hidden container

### B.2 Root Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

---

## C. Slot (Utility)

**Radix name:** `slot`
**Package:** `@radix-ui/react-slot`

### C.1 Anatomy (Parts)

- `Root` -- slot (merges props onto child)
- `Slottable` -- marks slottable children region

### C.2 Root Props

All standard HTML element props. Merges them onto its immediate child.

### C.3 Slottable

Wraps children that should receive slot props when multiple children exist.

---

## D. OneTimePasswordField (Preview)

**Import:** `import { unstable_OneTimePasswordField as OneTimePasswordField } from "radix-ui"`

### D.1 Anatomy (Parts)

- `Root` -- container
- `Input` -- single character input (render one per character)
- `HiddenInput` -- hidden input storing full value

### D.2 Root Props

| Prop           | Type                                               | Default         | Description                                 |
| -------------- | -------------------------------------------------- | --------------- | ------------------------------------------- |
| asChild        | boolean                                            | false           | Merge props onto child                      |
| autoComplete   | `"off" \| "one-time-code"`                         | "one-time-code" | Autocomplete hint                           |
| autoFocus      | boolean                                            | --              | Focus first input on load                   |
| value          | string                                             | --              | Controlled value                            |
| defaultValue   | string                                             | --              | Initial value                               |
| onValueChange  | `(value: string) => void`                          | --              | Value changed                               |
| autoSubmit     | boolean                                            | false           | Auto-submit on completion                   |
| onAutoSubmit   | `(value: string) => void`                          | --              | Called before auto-submit                   |
| disabled       | boolean                                            | false           | Disable inputs                              |
| dir            | `"ltr" \| "rtl"`                                   | "ltr"           | Reading direction                           |
| orientation    | `"horizontal" \| "vertical"`                       | "vertical"      | Layout orientation                          |
| form           | string                                             | --              | Associated form ID                          |
| name           | string                                             | --              | Form field name                             |
| placeholder    | string                                             | --              | Placeholder (split per character)           |
| readOnly       | boolean                                            | false           | Read-only                                   |
| sanitizeValue  | `(value: string) => string`                        | --              | Custom sanitization (validationType="none") |
| type           | `"text" \| "password"`                             | "text"          | Input type                                  |
| validationType | `"none" \| "numeric" \| "alpha" \| "alphanumeric"` | "numeric"       | Validation type                             |

### D.3 Input Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### D.4 HiddenInput Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### D.5 Data Attributes

| Part  | Attribute          | Values                   |
| ----- | ------------------ | ------------------------ |
| Root  | [data-orientation] | "vertical", "horizontal" |
| Input | [data-index]       | Character index          |

---

## E. PasswordToggleField (Preview)

**Import:** `import { unstable_PasswordToggleField as PasswordToggleField } from "radix-ui"`

### E.1 Anatomy (Parts)

- `Root` -- container
- `Input` -- password input
- `Toggle` -- visibility toggle button
- `Slot` -- conditional rendering slot
- `Icon` -- conditional icon rendering

### E.2 Root Props

| Prop              | Type                         | Default | Description                         |
| ----------------- | ---------------------------- | ------- | ----------------------------------- |
| id                | string                       | --      | Field ID (used for nested a11y IDs) |
| visible           | boolean                      | --      | Controlled visibility state         |
| defaultVisible    | boolean                      | --      | Initial visibility state            |
| onVisiblityChange | `(visible: boolean) => void` | --      | Visibility changed                  |

### E.3 Input Props

| Prop         | Type                                            | Default            | Description            |
| ------------ | ----------------------------------------------- | ------------------ | ---------------------- |
| asChild      | boolean                                         | false              | Merge props onto child |
| autoComplete | `"current-password" \| "new-password" \| "off"` | "current-password" | Autocomplete hint      |

### E.4 Toggle Props

| Prop    | Type    | Default | Description            |
| ------- | ------- | ------- | ---------------------- |
| asChild | boolean | false   | Merge props onto child |

### E.5 Slot Props

| Prop    | Type                                    | Default | Description          |
| ------- | --------------------------------------- | ------- | -------------------- |
| render  | `(props: { visible: boolean }) => void` | --      | Render function      |
| visible | ReactNode                               | --      | Content when visible |
| hidden  | ReactNode                               | --      | Content when hidden  |

### E.6 Icon Props

| Prop    | Type      | Default | Description                  |
| ------- | --------- | ------- | ---------------------------- |
| asChild | boolean   | false   | Merge props onto child       |
| visible | ReactNode | --      | Icon when visible (required) |
| hidden  | ReactNode | --      | Icon when hidden (required)  |

---

## F. Cross-Cutting Patterns

### F.1 asChild Prop

Every visual part in every component supports `asChild: boolean` (default false). When true, the component merges its props and behavior onto its immediate child element rather than rendering its default element. This is the foundation of Radix's composition model via the Slot utility.

### F.2 Positioning Props (Popper-based components)

Used by: Popover, Tooltip, HoverCard, DropdownMenu, ContextMenu, Menubar, Select (popper mode)

Common set: `side`, `sideOffset`, `align`, `alignOffset`, `avoidCollisions`, `collisionBoundary`, `collisionPadding`, `arrowPadding`, `sticky`, `hideWhenDetached`

### F.3 Dismiss Event Handlers

Used by: Dialog, AlertDialog, Popover, DropdownMenu, ContextMenu, Menubar, NavigationMenu, Tooltip, Select, Toast

Common set: `onEscapeKeyDown`, `onPointerDownOutside`, `onFocusOutside`, `onInteractOutside`

### F.4 Focus Management

Used by: Dialog, AlertDialog, Popover, DropdownMenu, ContextMenu, Menubar, Select

Common set: `onOpenAutoFocus`, `onCloseAutoFocus`

### F.5 Controlled/Uncontrolled Pattern

Most components support both patterns:

- `value`/`onValueChange` (or `open`/`onOpenChange`, `checked`/`onCheckedChange`, `pressed`/`onPressedChange`)
- `defaultValue` (or `defaultOpen`, `defaultChecked`, `defaultPressed`)

### F.6 forceMount Pattern

Available on: Portal, Overlay, Content, Indicator parts
Used for controlling mount/unmount when animating with React animation libraries.

### F.7 Portal Pattern

Components with overlays use a Portal part with `container` (HTMLElement, default document.body) and `forceMount` props.

### F.8 DirectionProvider

All components with `dir` prop inherit from a global `DirectionProvider` if dir is omitted.
