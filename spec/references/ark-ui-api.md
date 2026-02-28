---
title: Ark-UI API Reference
source: https://ark-ui.com/react/docs/components
secondary_source: https://zagjs.com/components/react
generated: 2026-03-27
total_components: 48
---

Ark-UI Component Library — Comprehensive API Reference

Research completed. All 48 Ark-UI components documented below, plus utilities. Data sourced from ark-ui.com React docs and zagjs.com.

---

## Table of Contents

1. [Accordion](#1-accordion)
2. [Angle Slider](#2-angle-slider)
3. [Avatar](#3-avatar)
4. [Carousel](#4-carousel)
5. [Checkbox](#5-checkbox)
6. [Clipboard](#6-clipboard)
7. [Collapsible](#7-collapsible)
8. [Color Picker](#8-color-picker)
9. [Combobox](#9-combobox)
10. [Date Picker](#10-date-picker)
11. [Dialog](#11-dialog)
12. [Editable](#12-editable)
13. [Field](#13-field)
14. [Fieldset](#14-fieldset)
15. [File Upload](#15-file-upload)
16. [Floating Panel](#16-floating-panel)
17. [Hover Card](#17-hover-card)
18. [Image Cropper](#18-image-cropper)
19. [Listbox](#19-listbox)
20. [Marquee](#20-marquee)
21. [Menu](#21-menu)
22. [Number Input](#22-number-input)
23. [Pagination](#23-pagination)
24. [Password Input](#24-password-input)
25. [Pin Input](#25-pin-input)
26. [Popover](#26-popover)
27. [Progress (Linear)](#27-progress-linear)
28. [Progress (Circular)](#28-progress-circular)
29. [QR Code](#29-qr-code)
30. [Radio Group](#30-radio-group)
31. [Rating Group](#31-rating-group)
32. [Scroll Area](#32-scroll-area)
33. [Segment Group](#33-segment-group)
34. [Select](#34-select)
35. [Signature Pad](#35-signature-pad)
36. [Slider](#36-slider)
37. [Splitter](#37-splitter)
38. [Steps](#38-steps)
39. [Switch](#39-switch)
40. [Tabs](#40-tabs)
41. [Tags Input](#41-tags-input)
42. [Timer](#42-timer)
43. [Toast](#43-toast)
44. [Toggle](#44-toggle)
45. [Toggle Group](#45-toggle-group)
46. [Tooltip](#46-tooltip)
47. [Tour](#47-tour)
48. [Tree View](#48-tree-view)
    A. [Utilities](#a-utilities)
    B. [Collections](#b-collections)
    C. [Components in Zag.js but NOT in Ark-UI](#c-components-in-zagjs-but-not-in-ark-ui)
    D. [Components in Our Spec NOT in Ark-UI](#d-components-in-our-spec-not-in-ark-ui)
    E. [Notes on Ark-UI Architecture](#e-notes-on-ark-ui-architecture)

---

## 1. Accordion

**Ark-UI name:** `Accordion`
**Category:** Navigation

### 1.1 Anatomy (Parts)

- `Accordion.Root` — main container (`<div>`)
- `Accordion.Item` — individual accordion item
- `Accordion.ItemTrigger` — clickable header (`<button>`)
- `Accordion.ItemIndicator` — visual indicator element
- `Accordion.ItemContent` — expandable content area
- `Accordion.RootProvider` — context provider alternative

### 1.2 Root Props

| Prop            | Type                                    | Default      | Description                               |
| --------------- | --------------------------------------- | ------------ | ----------------------------------------- |
| `collapsible`   | `boolean`                               | `false`      | Enables collapsing expanded items         |
| `defaultValue`  | `string[]`                              | —            | Initial expanded items                    |
| `disabled`      | `boolean`                               | —            | Disables all accordion items              |
| `id`            | `string`                                | —            | Unique identifier                         |
| `ids`           | `Partial<{...}>`                        | —            | Custom element IDs                        |
| `lazyMount`     | `boolean`                               | `false`      | Defers content rendering until expanded   |
| `multiple`      | `boolean`                               | `false`      | Allows multiple items open simultaneously |
| `onFocusChange` | `(details: FocusChangeDetails) => void` | —            | Fires when focused item changes           |
| `onValueChange` | `(details: ValueChangeDetails) => void` | —            | Fires when expanded items change          |
| `orientation`   | `'horizontal' \| 'vertical'`            | `'vertical'` | Layout direction                          |
| `unmountOnExit` | `boolean`                               | `false`      | Removes content when collapsed            |
| `value`         | `string[]`                              | —            | Controlled expanded items                 |
| `asChild`       | `boolean`                               | —            | Uses provided child as root element       |

### 1.3 Item Props

| Prop       | Type      | Default | Description                  |
| ---------- | --------- | ------- | ---------------------------- |
| `value`    | `string`  | —       | Unique identifier (required) |
| `disabled` | `boolean` | —       | Disables specific item       |
| `asChild`  | `boolean` | —       | Composition support          |

### 1.4 Data Attributes

| Part        | Attribute            | Values                       |
| ----------- | -------------------- | ---------------------------- |
| Root        | `[data-scope]`       | `"accordion"`                |
| Root        | `[data-part]`        | `"root"`                     |
| Root        | `[data-orientation]` | `"horizontal" \| "vertical"` |
| Item        | `[data-state]`       | `"open" \| "closed"`         |
| Item        | `[data-disabled]`    | Present when disabled        |
| Item        | `[data-focus]`       | Present when focused         |
| ItemTrigger | `[data-state]`       | `"open" \| "closed"`         |
| ItemTrigger | `[data-controls]`    | ID of controlled content     |
| ItemContent | `[data-state]`       | `"open" \| "closed"`         |
| ItemContent | `[data-disabled]`    | Present when disabled        |

### 1.5 Context API

| Property       | Type                        | Description              |
| -------------- | --------------------------- | ------------------------ |
| `value`        | `string[]`                  | Currently expanded items |
| `setValue`     | `(value: string[]) => void` | Update expanded items    |
| `getItemState` | `(props) => ItemState`      | Get item state           |

---

## 2. Angle Slider

**Ark-UI name:** `AngleSlider`
**Category:** Specialized

### 2.1 Anatomy (Parts)

- `AngleSlider.Root` — main container
- `AngleSlider.Label` — label element
- `AngleSlider.Control` — circular control area
- `AngleSlider.Thumb` — draggable thumb
- `AngleSlider.MarkerGroup` — container for markers
- `AngleSlider.Marker` — individual angle marker
- `AngleSlider.ValueText` — displays current value
- `AngleSlider.HiddenInput` — hidden form input
- `AngleSlider.RootProvider` — context provider

### 2.2 Root Props

| Prop               | Type                                                             | Default | Description                       |
| ------------------ | ---------------------------------------------------------------- | ------- | --------------------------------- |
| `aria-label`       | `string`                                                         | —       | Accessible label for slider thumb |
| `aria-labelledby`  | `string`                                                         | —       | ID of labeling element            |
| `asChild`          | `boolean`                                                        | —       | Use child as rendered element     |
| `defaultValue`     | `number`                                                         | `0`     | Initial slider value              |
| `disabled`         | `boolean`                                                        | —       | Disable the slider                |
| `id`               | `string`                                                         | —       | Unique machine identifier         |
| `ids`              | `Partial<{root, thumb, hiddenInput, control, valueText, label}>` | —       | Element IDs                       |
| `invalid`          | `boolean`                                                        | —       | Mark as invalid                   |
| `name`             | `string`                                                         | —       | Form submission name              |
| `onValueChange`    | `(details: ValueChangeDetails) => void`                          | —       | Value change callback             |
| `onValueChangeEnd` | `(details: ValueChangeDetails) => void`                          | —       | Change completion callback        |
| `readOnly`         | `boolean`                                                        | —       | Read-only mode                    |
| `step`             | `number`                                                         | `1`     | Discrete step intervals           |
| `value`            | `number`                                                         | —       | Controlled value                  |

### 2.3 Marker Props

| Prop      | Type      | Description                |
| --------- | --------- | -------------------------- |
| `value`   | `number`  | Marker position (required) |
| `asChild` | `boolean` | Composition support        |

### 2.4 Data Attributes

| Part   | Attribute         | Values                 |
| ------ | ----------------- | ---------------------- |
| Root   | `[data-scope]`    | `"angle-slider"`       |
| Root   | `[data-disabled]` | Present when disabled  |
| Root   | `[data-invalid]`  | Present when invalid   |
| Root   | `[data-readonly]` | Present when read-only |
| Marker | `[data-value]`    | Marker numeric value   |
| Marker | `[data-state]`    | State indicator        |

### 2.5 CSS Variables

| Variable                 | Description                |
| ------------------------ | -------------------------- |
| `--value`                | Current numeric value      |
| `--angle`                | Angle in degrees           |
| `--marker-value`         | Logical marker value       |
| `--marker-display-value` | Rotation angle (RTL-aware) |

---

## 3. Avatar

**Ark-UI name:** `Avatar`
**Category:** Data Display

### 3.1 Anatomy (Parts)

- `Avatar.Root` — container (`<div>`)
- `Avatar.Image` — image element (`<img>`)
- `Avatar.Fallback` — displayed when image fails (`<span>`)
- `Avatar.RootProvider` — context provider

### 3.2 Root Props

| Prop             | Type                                     | Default | Description                 |
| ---------------- | ---------------------------------------- | ------- | --------------------------- |
| `asChild`        | `boolean`                                | —       | Use child as root element   |
| `ids`            | `Partial<{root, image, fallback}>`       | —       | Custom element IDs          |
| `onStatusChange` | `(details: StatusChangeDetails) => void` | —       | Image loading status change |

### 3.3 Data Attributes

| Part     | Attribute      | Values                  |
| -------- | -------------- | ----------------------- |
| Image    | `[data-scope]` | `"avatar"`              |
| Image    | `[data-state]` | `"visible" \| "hidden"` |
| Fallback | `[data-state]` | `"visible" \| "hidden"` |

### 3.4 Context API

| Property    | Type                    | Description             |
| ----------- | ----------------------- | ----------------------- |
| `loaded`    | `boolean`               | Whether image is loaded |
| `setSrc`    | `(src: string) => void` | Update image source     |
| `setLoaded` | `VoidFunction`          | Mark as loaded          |
| `setError`  | `VoidFunction`          | Mark as errored         |

---

## 4. Carousel

**Ark-UI name:** `Carousel`
**Category:** Layout

### 4.1 Anatomy (Parts)

- `Carousel.Root` — main container
- `Carousel.Control` — control wrapper
- `Carousel.PrevTrigger` — previous button
- `Carousel.NextTrigger` — next button
- `Carousel.ItemGroup` — slide container
- `Carousel.Item` — individual slide
- `Carousel.IndicatorGroup` — indicator container
- `Carousel.Indicator` — page indicator
- `Carousel.AutoplayTrigger` — autoplay toggle
- `Carousel.AutoplayIndicator` — autoplay state indicator
- `Carousel.ProgressText` — progress text
- `Carousel.RootProvider` — context provider

### 4.2 Root Props

| Prop                     | Type                                       | Default        | Description                        |
| ------------------------ | ------------------------------------------ | -------------- | ---------------------------------- |
| `slideCount`             | `number`                                   | —              | Total slides for SSR               |
| `allowMouseDrag`         | `boolean`                                  | `false`        | Enable mouse drag scrolling        |
| `asChild`                | `boolean`                                  | —              | Use child as root                  |
| `autoplay`               | `boolean \| { delay: number }`             | `false`        | Auto-scroll (default delay 4000ms) |
| `autoSize`               | `boolean`                                  | `false`        | Allow variable-width slides        |
| `defaultPage`            | `number`                                   | `0`            | Initial page on render             |
| `ids`                    | `Partial<{...}>`                           | —              | Custom element IDs                 |
| `inViewThreshold`        | `number \| number[]`                       | `0.6`          | In-view detection threshold        |
| `loop`                   | `boolean`                                  | `false`        | Enable looping                     |
| `onAutoplayStatusChange` | `(details: AutoplayStatusDetails) => void` | —              | Autoplay status callback           |
| `onDragStatusChange`     | `(details: DragStatusDetails) => void`     | —              | Drag status callback               |
| `onPageChange`           | `(details: PageChangeDetails) => void`     | —              | Page change callback               |
| `orientation`            | `'horizontal' \| 'vertical'`               | `'horizontal'` | Scroll direction                   |
| `padding`                | `string`                                   | —              | Extra scrollable area              |
| `page`                   | `number`                                   | —              | Controlled page index              |
| `slidesPerMove`          | `number \| 'auto'`                         | `'auto'`       | Slides per scroll action           |
| `slidesPerPage`          | `number`                                   | `1`            | Visible slides                     |
| `snapType`               | `'proximity' \| 'mandatory'`               | `'mandatory'`  | Snap behavior                      |
| `spacing`                | `string`                                   | `'0px'`        | Gap between slides                 |
| `translations`           | `IntlTranslations`                         | —              | Localization messages              |

### 4.3 Item Props

| Prop        | Type                           | Default   | Description            |
| ----------- | ------------------------------ | --------- | ---------------------- |
| `index`     | `number`                       | —         | Slide index (required) |
| `asChild`   | `boolean`                      | —         | Composition support    |
| `snapAlign` | `'center' \| 'start' \| 'end'` | `'start'` | Snap alignment         |

### 4.4 Data Attributes

| Part      | Attribute            | Values                       |
| --------- | -------------------- | ---------------------------- |
| Root      | `[data-orientation]` | `"horizontal" \| "vertical"` |
| Item      | `[data-inview]`      | Present when visible         |
| Item      | `[data-index]`       | Item index                   |
| Indicator | `[data-current]`     | Present when active          |
| ItemGroup | `[data-dragging]`    | Present during drag          |

### 4.5 CSS Variables

| Variable            | Description           |
| ------------------- | --------------------- |
| `--slides-per-page` | Visible slide count   |
| `--slide-spacing`   | Inter-slide gap       |
| `--slide-item-size` | Calculated item width |

### 4.6 Context API

| Property         | Type                                         | Description                   |
| ---------------- | -------------------------------------------- | ----------------------------- |
| `page`           | `number`                                     | Current page                  |
| `pageSnapPoints` | `number[]`                                   | Snap points                   |
| `isPlaying`      | `boolean`                                    | Autoplay state                |
| `isDragging`     | `boolean`                                    | Drag state                    |
| `canScrollNext`  | `boolean`                                    | Forward navigation available  |
| `canScrollPrev`  | `boolean`                                    | Backward navigation available |
| `scrollToIndex`  | `(index: number, instant?: boolean) => void` | Scroll to slide               |
| `scrollTo`       | `(page: number, instant?: boolean) => void`  | Scroll to page                |
| `scrollNext`     | `(instant?: boolean) => void`                | Next page                     |
| `scrollPrev`     | `(instant?: boolean) => void`                | Previous page                 |
| `getProgress`    | `() => number`                               | Progress value                |
| `play`           | `VoidFunction`                               | Start autoplay                |
| `pause`          | `VoidFunction`                               | Stop autoplay                 |
| `isInView`       | `(index: number) => boolean`                 | Check visibility              |
| `refresh`        | `VoidFunction`                               | Refresh layout                |

---

## 5. Checkbox

**Ark-UI name:** `Checkbox`
**Category:** Input

### 5.1 Anatomy (Parts)

- `Checkbox.Root` — main container
- `Checkbox.Control` — visual checkbox
- `Checkbox.Indicator` — check/indeterminate icon
- `Checkbox.Label` — label text
- `Checkbox.HiddenInput` — hidden form input
- `Checkbox.Group` — group container
- `Checkbox.RootProvider` — context provider

### 5.2 Root Props

| Prop              | Type                                           | Default | Description              |
| ----------------- | ---------------------------------------------- | ------- | ------------------------ |
| `asChild`         | `boolean`                                      | —       | Composition support      |
| `checked`         | `CheckedState`                                 | —       | Controlled checked state |
| `defaultChecked`  | `CheckedState`                                 | —       | Initial checked state    |
| `disabled`        | `boolean`                                      | —       | Non-interactive          |
| `form`            | `string`                                       | —       | Associated form ID       |
| `id`              | `string`                                       | —       | Unique identifier        |
| `ids`             | `Partial<{root, hiddenInput, control, label}>` | —       | Custom IDs               |
| `invalid`         | `boolean`                                      | —       | Invalid state            |
| `name`            | `string`                                       | —       | Form field name          |
| `onCheckedChange` | `(details: CheckedChangeDetails) => void`      | —       | Change callback          |
| `readOnly`        | `boolean`                                      | —       | Read-only                |
| `required`        | `boolean`                                      | —       | Required field           |
| `value`           | `string`                                       | `"on"`  | Form submission value    |

### 5.3 Group Props

| Prop                | Type                | Default | Description             |
| ------------------- | ------------------- | ------- | ----------------------- |
| `defaultValue`      | `string[]`          | —       | Initial selected values |
| `disabled`          | `boolean`           | —       | Disables group          |
| `invalid`           | `boolean`           | —       | Invalid state           |
| `maxSelectedValues` | `number`            | —       | Max selectable          |
| `name`              | `string`            | —       | Form field name         |
| `onValueChange`     | `(details) => void` | —       | Change callback         |
| `readOnly`          | `boolean`           | —       | Read-only               |
| `value`             | `string[]`          | —       | Controlled values       |

### 5.4 Indicator Props

| Prop            | Type      | Description                 |
| --------------- | --------- | --------------------------- |
| `indeterminate` | `boolean` | Renders indeterminate state |
| `asChild`       | `boolean` | Composition support         |

### 5.5 Data Attributes

| Attribute              | Values                                        |
| ---------------------- | --------------------------------------------- |
| `[data-active]`        | Present when active                           |
| `[data-focus]`         | Present when focused                          |
| `[data-focus-visible]` | Present on keyboard focus                     |
| `[data-readonly]`      | Present when read-only                        |
| `[data-hover]`         | Present on hover                              |
| `[data-disabled]`      | Present when disabled                         |
| `[data-state]`         | `"indeterminate" \| "checked" \| "unchecked"` |
| `[data-invalid]`       | Present when invalid                          |
| `[data-required]`      | Present when required                         |

### 5.6 Context API

| Property        | Type                              | Description              |
| --------------- | --------------------------------- | ------------------------ |
| `checked`       | `boolean`                         | Current checked state    |
| `disabled`      | `boolean`                         | Disabled status          |
| `indeterminate` | `boolean`                         | Indeterminate state      |
| `focused`       | `boolean`                         | Focus status             |
| `checkedState`  | `CheckedState`                    | Full checked state value |
| `setChecked`    | `(checked: CheckedState) => void` | Update state             |
| `toggleChecked` | `VoidFunction`                    | Toggle state             |

---

## 6. Clipboard

**Ark-UI name:** `Clipboard`
**Category:** Specialized

### 6.1 Anatomy (Parts)

- `Clipboard.Root` — container
- `Clipboard.Label` — label
- `Clipboard.Control` — control wrapper
- `Clipboard.Input` — input field
- `Clipboard.Trigger` — copy button
- `Clipboard.Indicator` — copy status indicator
- `Clipboard.ValueText` — displays value
- `Clipboard.RootProvider` — context provider

### 6.2 Root Props

| Prop             | Type                                     | Default | Description              |
| ---------------- | ---------------------------------------- | ------- | ------------------------ |
| `asChild`        | `boolean`                                | —       | Composition support      |
| `defaultValue`   | `string`                                 | —       | Initial clipboard value  |
| `id`             | `string`                                 | —       | Unique identifier        |
| `ids`            | `Partial<{...}>`                         | —       | Custom element IDs       |
| `onStatusChange` | `(details: StatusChangeDetails) => void` | —       | Copy status callback     |
| `onValueChange`  | `(details: ValueChangeDetails) => void`  | —       | Value change callback    |
| `timeout`        | `number`                                 | `3000`  | Copy status timeout (ms) |
| `value`          | `string`                                 | —       | Controlled value         |

### 6.3 Indicator Props

| Prop      | Type        | Description                   |
| --------- | ----------- | ----------------------------- |
| `copied`  | `ReactNode` | Content shown in copied state |
| `asChild` | `boolean`   | Composition support           |

### 6.4 Data Attributes

| Attribute       | Values             |
| --------------- | ------------------ |
| `[data-scope]`  | `"clipboard"`      |
| `[data-copied]` | Present after copy |

### 6.5 Context API

| Property   | Type                      | Description        |
| ---------- | ------------------------- | ------------------ |
| `copied`   | `boolean`                 | Copy success state |
| `value`    | `string`                  | Current value      |
| `setValue` | `(value: string) => void` | Update value       |
| `copy`     | `VoidFunction`            | Execute copy       |

---

## 7. Collapsible

**Ark-UI name:** `Collapsible`
**Category:** Layout

### 7.1 Anatomy (Parts)

- `Collapsible.Root` — container (`<div>`)
- `Collapsible.Trigger` — toggle button (`<button>`)
- `Collapsible.Indicator` — visual indicator (`<div>`)
- `Collapsible.Content` — expandable content (`<div>`)
- `Collapsible.RootProvider` — context provider

### 7.2 Root Props

| Prop              | Type                                   | Default | Description             |
| ----------------- | -------------------------------------- | ------- | ----------------------- |
| `asChild`         | `boolean`                              | —       | Composition support     |
| `collapsedHeight` | `string \| number`                     | —       | Height when collapsed   |
| `collapsedWidth`  | `string \| number`                     | —       | Width when collapsed    |
| `defaultOpen`     | `boolean`                              | —       | Initial open state      |
| `disabled`        | `boolean`                              | —       | Disables toggling       |
| `ids`             | `Partial<{root, content, trigger}>`    | —       | Custom IDs              |
| `lazyMount`       | `boolean`                              | `false` | Delay mounting          |
| `onExitComplete`  | `VoidFunction`                         | —       | Exit animation callback |
| `onOpenChange`    | `(details: OpenChangeDetails) => void` | —       | Open state callback     |
| `open`            | `boolean`                              | —       | Controlled state        |
| `unmountOnExit`   | `boolean`                              | `false` | Remove from DOM         |

### 7.3 Data Attributes

| Part                           | Attribute                   | Values                          |
| ------------------------------ | --------------------------- | ------------------------------- |
| Root/Content/Trigger/Indicator | `[data-state]`              | `"open" \| "closed"`            |
| Content                        | `[data-disabled]`           | Present when disabled           |
| Content                        | `[data-has-collapsed-size]` | Present when collapsed size set |

### 7.4 CSS Variables (Content)

| Variable             | Description      |
| -------------------- | ---------------- |
| `--height`           | Element height   |
| `--width`            | Element width    |
| `--collapsed-height` | Collapsed height |
| `--collapsed-width`  | Collapsed width  |

### 7.5 Context API

| Property      | Type                      | Description             |
| ------------- | ------------------------- | ----------------------- |
| `open`        | `boolean`                 | Open state              |
| `visible`     | `boolean`                 | Accounts for animations |
| `disabled`    | `boolean`                 | Disabled state          |
| `setOpen`     | `(open: boolean) => void` | Toggle open             |
| `measureSize` | `VoidFunction`            | Measure dimensions      |

---

## 8. Color Picker

**Ark-UI name:** `ColorPicker`
**Category:** Specialized

### 8.1 Anatomy (Parts)

- `ColorPicker.Root` — main container
- `ColorPicker.Label` — label
- `ColorPicker.Control` — control wrapper
- `ColorPicker.ChannelInput` — channel text input (`<input>`)
- `ColorPicker.Trigger` — open trigger
- `ColorPicker.ValueSwatch` — current color swatch
- `ColorPicker.Positioner` — positioning wrapper
- `ColorPicker.Content` — popup content
- `ColorPicker.Area` — 2D color area
- `ColorPicker.AreaBackground` — area background
- `ColorPicker.AreaThumb` — area thumb
- `ColorPicker.EyeDropperTrigger` — eye dropper button
- `ColorPicker.ChannelSlider` — channel slider
- `ColorPicker.ChannelSliderTrack` — slider track
- `ColorPicker.ChannelSliderThumb` — slider thumb
- `ColorPicker.ChannelSliderLabel` — slider label
- `ColorPicker.ChannelSliderValueText` — slider value text
- `ColorPicker.SwatchGroup` — swatch group container
- `ColorPicker.SwatchTrigger` — swatch select button
- `ColorPicker.Swatch` — individual swatch
- `ColorPicker.SwatchIndicator` — swatch selection indicator
- `ColorPicker.HiddenInput` — hidden form input
- `ColorPicker.TransparencyGrid` — alpha grid background
- `ColorPicker.ValueText` — formatted value text
- `ColorPicker.FormatSelect` — format dropdown (`<select>`)
- `ColorPicker.FormatTrigger` — format cycle button
- `ColorPicker.View` — format-specific view
- `ColorPicker.RootProvider` — context provider

### 8.2 Root Props

| Prop                   | Type                        | Default   | Description                  |
| ---------------------- | --------------------------- | --------- | ---------------------------- |
| `value`                | `Color`                     | —         | Controlled color value       |
| `defaultValue`         | `Color`                     | `#000000` | Initial color                |
| `format`               | `ColorFormat`               | —         | Controlled color format      |
| `defaultFormat`        | `ColorFormat`               | `'rgba'`  | Initial format               |
| `open`                 | `boolean`                   | —         | Controlled open state        |
| `defaultOpen`          | `boolean`                   | —         | Initial open state           |
| `disabled`             | `boolean`                   | —         | Disables interaction         |
| `readOnly`             | `boolean`                   | —         | Read-only mode               |
| `invalid`              | `boolean`                   | —         | Invalid state                |
| `required`             | `boolean`                   | —         | Required field               |
| `inline`               | `boolean`                   | —         | Render without popover       |
| `closeOnSelect`        | `boolean`                   | `false`   | Close after swatch selection |
| `name`                 | `string`                    | —         | Form input name              |
| `id`                   | `string`                    | —         | Unique identifier            |
| `openAutoFocus`        | `boolean`                   | `true`    | Auto-focus on open           |
| `lazyMount`            | `boolean`                   | `false`   | Lazy mounting                |
| `unmountOnExit`        | `boolean`                   | `false`   | Unmount when closed          |
| `skipAnimationOnMount` | `boolean`                   | `false`   | Skip initial animation       |
| `positioning`          | `PositioningOptions`        | —         | Popover positioning          |
| `immediate`            | `boolean`                   | —         | Sync changes immediately     |
| `initialFocusEl`       | `() => HTMLElement \| null` | —         | Initial focus element        |

### 8.3 Events

| Event                  | Payload                   | Description          |
| ---------------------- | ------------------------- | -------------------- |
| `onValueChange`        | `ValueChangeDetails`      | While dragging       |
| `onValueChangeEnd`     | `ValueChangeDetails`      | After drag completes |
| `onOpenChange`         | `OpenChangeDetails`       | Popup state changes  |
| `onFormatChange`       | `FormatChangeDetails`     | Format changed       |
| `onFocusOutside`       | `FocusOutsideEvent`       | External focus       |
| `onInteractOutside`    | `InteractOutsideEvent`    | External interaction |
| `onPointerDownOutside` | `PointerDownOutsideEvent` | Pointer down outside |
| `onExitComplete`       | —                         | Animation end        |

### 8.4 ChannelInput Props

| Prop          | Type                   | Description              |
| ------------- | ---------------------- | ------------------------ |
| `channel`     | `ExtendedColorChannel` | Color channel (required) |
| `orientation` | `Orientation`          | Layout direction         |
| `asChild`     | `boolean`              | Composition              |

### 8.5 SwatchTrigger Props

| Prop       | Type              | Description            |
| ---------- | ----------------- | ---------------------- |
| `value`    | `string \| Color` | Color value (required) |
| `disabled` | `boolean`         | Disabled state         |
| `asChild`  | `boolean`         | Composition            |

### 8.6 Swatch Props

| Prop           | Type              | Description            |
| -------------- | ----------------- | ---------------------- |
| `value`        | `string \| Color` | Color value (required) |
| `respectAlpha` | `boolean`         | Include alpha channel  |
| `asChild`      | `boolean`         | Composition            |

### 8.7 ValueSwatch Props

| Prop           | Type      | Description   |
| -------------- | --------- | ------------- |
| `respectAlpha` | `boolean` | Include alpha |
| `asChild`      | `boolean` | Composition   |

### 8.8 TransparencyGrid Props

| Prop      | Type      | Description    |
| --------- | --------- | -------------- |
| `size`    | `string`  | Grid cell size |
| `asChild` | `boolean` | Composition    |

### 8.9 View Props

| Prop      | Type          | Description          |
| --------- | ------------- | -------------------- |
| `format`  | `ColorFormat` | Color format to show |
| `asChild` | `boolean`     | Composition          |

### 8.10 Data Attributes

| Attribute            | Values                 |
| -------------------- | ---------------------- |
| `[data-scope]`       | `"color-picker"`       |
| `[data-state]`       | `"open" \| "closed"`   |
| `[data-disabled]`    | Present when disabled  |
| `[data-readonly]`    | Present when read-only |
| `[data-invalid]`     | Present when invalid   |
| `[data-required]`    | Present when required  |
| `[data-focus]`       | Present when focused   |
| `[data-channel]`     | Channel name           |
| `[data-orientation]` | Slider orientation     |
| `[data-value]`       | Swatch color value     |

### 8.11 CSS Variables

| Variable        | Description                  |
| --------------- | ---------------------------- |
| `--value`       | Current color value          |
| `--color`       | Text/swatch color            |
| `--size`        | Grid size (TransparencyGrid) |
| `--layer-index` | Stack index                  |

---

## 9. Combobox

**Ark-UI name:** `Combobox`
**Category:** Selection

### 9.1 Anatomy (Parts)

- `Combobox.Root` — main container
- `Combobox.Label` — label
- `Combobox.Control` — control wrapper
- `Combobox.Input` — text input
- `Combobox.Trigger` — open trigger
- `Combobox.ClearTrigger` — clear button
- `Combobox.Positioner` — positioning wrapper
- `Combobox.Content` — dropdown content
- `Combobox.ItemGroup` — item group
- `Combobox.ItemGroupLabel` — group label
- `Combobox.Item` — individual option
- `Combobox.ItemText` — item text
- `Combobox.ItemIndicator` — selection indicator
- `Combobox.Empty` — empty state
- `Combobox.List` — virtualized list
- `Combobox.RootProvider` — context provider

### 9.2 Root Props

| Prop                      | Type                                          | Default                         | Description                        |
| ------------------------- | --------------------------------------------- | ------------------------------- | ---------------------------------- |
| `collection`              | `ListCollection<T>`                           | —                               | Item collection (required)         |
| `allowCustomValue`        | `boolean`                                     | —                               | Allow typing custom values         |
| `alwaysSubmitOnEnter`     | `boolean`                                     | `false`                         | Submit on Enter even if popup open |
| `autoFocus`               | `boolean`                                     | —                               | Autofocus input on mount           |
| `closeOnSelect`           | `boolean`                                     | —                               | Close when item selected           |
| `composite`               | `boolean`                                     | `true`                          | Composed with other widgets        |
| `defaultHighlightedValue` | `string`                                      | —                               | Initial highlighted value          |
| `defaultInputValue`       | `string`                                      | `''`                            | Initial input value                |
| `defaultOpen`             | `boolean`                                     | —                               | Initial open state                 |
| `defaultValue`            | `string[]`                                    | `[]`                            | Initial selected items             |
| `disabled`                | `boolean`                                     | —                               | Disable combobox                   |
| `highlightedValue`        | `string`                                      | —                               | Controlled highlighted value       |
| `inputBehavior`           | `'none' \| 'autohighlight' \| 'autocomplete'` | `'none'`                        | Auto-completion behavior           |
| `inputValue`              | `string`                                      | —                               | Controlled input value             |
| `invalid`                 | `boolean`                                     | —                               | Mark as invalid                    |
| `loopFocus`               | `boolean`                                     | `true`                          | Loop keyboard navigation           |
| `multiple`                | `boolean`                                     | —                               | Multiple selection                 |
| `name`                    | `string`                                      | —                               | Input name                         |
| `open`                    | `boolean`                                     | —                               | Controlled open state              |
| `openOnChange`            | `boolean \| function`                         | `true`                          | Show on input changes              |
| `openOnClick`             | `boolean`                                     | `false`                         | Open on input click                |
| `openOnKeyPress`          | `boolean`                                     | `true`                          | Open on arrow key                  |
| `placeholder`             | `string`                                      | —                               | Input placeholder                  |
| `positioning`             | `PositioningOptions`                          | `{ placement: 'bottom-start' }` | Positioning options                |
| `readOnly`                | `boolean`                                     | —                               | Read-only                          |
| `required`                | `boolean`                                     | —                               | Required                           |
| `selectionBehavior`       | `'clear' \| 'replace' \| 'preserve'`          | `'replace'`                     | Input behavior on selection        |
| `value`                   | `string[]`                                    | —                               | Controlled selected items          |

### 9.3 Events

| Event                | Payload                     | Description         |
| -------------------- | --------------------------- | ------------------- |
| `onHighlightChange`  | `HighlightChangeDetails<T>` | Item highlighted    |
| `onInputValueChange` | `InputValueChangeDetails`   | Input value changed |
| `onOpenChange`       | `OpenChangeDetails`         | Popup opened/closed |
| `onValueChange`      | `ValueChangeDetails<T>`     | Selection changed   |
| `onSelect`           | `SelectionDetails`          | Item selected       |

### 9.4 Item Props

| Prop           | Type      | Description                      |
| -------------- | --------- | -------------------------------- |
| `item`         | `any`     | Item data                        |
| `persistFocus` | `boolean` | Preserve highlight on mouse exit |
| `asChild`      | `boolean` | Composition                      |

### 9.5 Data Attributes

| Part    | Attribute            | Values                     |
| ------- | -------------------- | -------------------------- |
| Root    | `[data-invalid]`     | Present when invalid       |
| Root    | `[data-readonly]`    | Present when read-only     |
| Item    | `[data-highlighted]` | Present when focused       |
| Item    | `[data-disabled]`    | Present when disabled      |
| Item    | `[data-state]`       | `"checked" \| "unchecked"` |
| Content | `[data-placement]`   | Positioning direction      |

### 9.6 Context API

| Property           | Type                               | Description      |
| ------------------ | ---------------------------------- | ---------------- |
| `focused`          | `boolean`                          | Focus state      |
| `open`             | `boolean`                          | Open state       |
| `inputValue`       | `string`                           | Input text       |
| `highlightedValue` | `string`                           | Highlighted item |
| `selectedItems`    | `V[]`                              | Selected items   |
| `value`            | `string[]`                         | Selected keys    |
| `setInputValue`    | `(value: string, reason?) => void` | Update input     |
| `selectValue`      | `(value: string) => void`          | Select item      |
| `clearValue`       | `(value?: string) => void`         | Clear selection  |
| `setOpen`          | `(open: boolean, reason?) => void` | Control menu     |

---

## 10. Date Picker

**Ark-UI name:** `DatePicker`
**Category:** Date-Time

### 10.1 Anatomy (Parts)

- `DatePicker.Root`, `.Label`, `.Control`, `.Input`, `.Trigger`, `.ClearTrigger`
- `DatePicker.Positioner`, `.Content`
- `DatePicker.View`, `.ViewControl`, `.PrevTrigger`, `.ViewTrigger`, `.RangeText`, `.NextTrigger`
- `DatePicker.Table`, `.TableHead`, `.TableRow`, `.TableHeader`, `.TableBody`, `.TableCell`, `.TableCellTrigger`
- `DatePicker.MonthSelect`, `.YearSelect`
- `DatePicker.PresetTrigger`, `.ValueText`
- `DatePicker.WeekNumberCell`, `.WeekNumberHeaderCell`
- `DatePicker.RootProvider`

### 10.2 Root Props

| Prop                   | Type                                                                | Default    | Description               |
| ---------------------- | ------------------------------------------------------------------- | ---------- | ------------------------- |
| `closeOnSelect`        | `boolean`                                                           | `true`     | Close after selection     |
| `defaultValue`         | `DateValue[]`                                                       | —          | Initial selected dates    |
| `defaultFocusedValue`  | `DateValue`                                                         | —          | Initial focused date      |
| `defaultOpen`          | `boolean`                                                           | —          | Initial open state        |
| `defaultView`          | `'day' \| 'month' \| 'year'`                                        | `'day'`    | Starting view             |
| `disabled`             | `boolean`                                                           | —          | Disables calendar         |
| `fixedWeeks`           | `boolean`                                                           | —          | Always 6 weeks            |
| `focusedValue`         | `DateValue`                                                         | —          | Controlled focused date   |
| `format`               | `(date: DateValue, details: LocaleDetails) => string`               | —          | Custom format function    |
| `id`                   | `string`                                                            | —          | Unique identifier         |
| `ids`                  | `Partial<{...}>`                                                    | —          | Element IDs               |
| `inline`               | `boolean`                                                           | —          | Render without popup      |
| `invalid`              | `boolean`                                                           | —          | Invalid state             |
| `isDateUnavailable`    | `(date: DateValue, locale: string) => boolean`                      | —          | Unavailable dates         |
| `lazyMount`            | `boolean`                                                           | `false`    | Lazy mounting             |
| `locale`               | `string`                                                            | `'en-US'`  | BCP 47 locale             |
| `max`                  | `DateValue`                                                         | —          | Maximum date              |
| `maxSelectedDates`     | `number`                                                            | —          | Max for multiple mode     |
| `maxView`              | `'day' \| 'month' \| 'year'`                                        | `'year'`   | Maximum view              |
| `min`                  | `DateValue`                                                         | —          | Minimum date              |
| `minView`              | `'day' \| 'month' \| 'year'`                                        | `'day'`    | Minimum view              |
| `name`                 | `string`                                                            | —          | Input name                |
| `numOfMonths`          | `number`                                                            | —          | Side-by-side months       |
| `open`                 | `boolean`                                                           | —          | Controlled open           |
| `openOnClick`          | `boolean`                                                           | `false`    | Open on input click       |
| `outsideDaySelectable` | `boolean`                                                           | `false`    | Select outside-month days |
| `parse`                | `(value: string, details: LocaleDetails) => DateValue \| undefined` | —          | Custom parsing            |
| `placeholder`          | `string`                                                            | —          | Input placeholder         |
| `positioning`          | `PositioningOptions`                                                | —          | Content positioning       |
| `readOnly`             | `boolean`                                                           | —          | Read-only                 |
| `required`             | `boolean`                                                           | —          | Required                  |
| `selectionMode`        | `'single' \| 'multiple' \| 'range'`                                 | `'single'` | Selection behavior        |
| `showWeekNumbers`      | `boolean`                                                           | —          | Display week numbers      |
| `startOfWeek`          | `number`                                                            | —          | First day (0=Sun)         |
| `timeZone`             | `string`                                                            | `'UTC'`    | Timezone                  |
| `translations`         | `IntlTranslations`                                                  | —          | i18n messages             |
| `unmountOnExit`        | `boolean`                                                           | `false`    | Unmount on exit           |
| `value`                | `DateValue[]`                                                       | —          | Controlled dates          |
| `view`                 | `DateView`                                                          | —          | Controlled view           |

### 10.3 Events

| Event                  | Payload                     | Description           |
| ---------------------- | --------------------------- | --------------------- |
| `onValueChange`        | `ValueChangeDetails`        | Date(s) changed       |
| `onFocusChange`        | `FocusChangeDetails`        | Focused date changed  |
| `onOpenChange`         | `OpenChangeDetails`         | Open/close            |
| `onViewChange`         | `ViewChangeDetails`         | View changed          |
| `onVisibleRangeChange` | `VisibleRangeChangeDetails` | Visible range changed |
| `onExitComplete`       | —                           | Animation done        |

### 10.4 Input Props

| Prop        | Type      | Default | Description                |
| ----------- | --------- | ------- | -------------------------- |
| `fixOnBlur` | `boolean` | `true`  | Correct value on blur      |
| `index`     | `number`  | —       | Input index for multi-date |
| `asChild`   | `boolean` | —       | Composition                |

### 10.5 Data Attributes

| Part             | Attribute                  | Values                           |
| ---------------- | -------------------------- | -------------------------------- |
| Root             | `[data-state]`             | `"open" \| "closed"`             |
| Root             | `[data-disabled]`          | Present when disabled            |
| Root             | `[data-readonly]`          | Present when read-only           |
| Root             | `[data-empty]`             | Present when no selection        |
| Input            | `[data-placeholder-shown]` | Present when showing placeholder |
| TableCellTrigger | `[data-today]`             | Present for today                |
| TableCellTrigger | `[data-selected]`          | Present when selected            |
| TableCellTrigger | `[data-in-range]`          | Present in range selection       |
| TableCellTrigger | `[data-range-start]`       | Range start                      |
| TableCellTrigger | `[data-range-end]`         | Range end                        |
| TableCellTrigger | `[data-unavailable]`       | Unavailable date                 |
| TableCellTrigger | `[data-outside-range]`     | Outside visible range            |

### 10.6 Context API

| Property          | Type                            | Description          |
| ----------------- | ------------------------------- | -------------------- |
| `value`           | `DateValue[]`                   | Selected dates       |
| `valueAsDate`     | `Date[]`                        | As Date objects      |
| `valueAsString`   | `string[]`                      | As formatted strings |
| `focusedValue`    | `DateValue`                     | Currently focused    |
| `view`            | `DateView`                      | Current view         |
| `weeks`           | `DateValue[][]`                 | Month grid           |
| `weekDays`        | `WeekDay[]`                     | Day names            |
| `visibleRange`    | `VisibleRange`                  | Visible date range   |
| `open`            | `boolean`                       | Open state           |
| `selectToday`     | `VoidFunction`                  | Select today         |
| `setValue`        | `(values: DateValue[]) => void` | Update selection     |
| `setFocusedValue` | `(value: DateValue) => void`    | Focus date           |
| `clearValue`      | `(options?) => void`            | Clear selection      |
| `setOpen`         | `(open: boolean) => void`       | Control open         |
| `setView`         | `(view: DateView) => void`      | Change view          |
| `goToNext`        | `VoidFunction`                  | Navigate forward     |
| `goToPrev`        | `VoidFunction`                  | Navigate backward    |

---

## 11. Dialog

**Ark-UI name:** `Dialog`
**Category:** Overlay

### 11.1 Anatomy (Parts)

- `Dialog.Root` — main container
- `Dialog.Trigger` — open button (`<button>`)
- `Dialog.Backdrop` — overlay (`<div>`)
- `Dialog.Positioner` — positioning wrapper (`<div>`)
- `Dialog.Content` — main content (`<div>`)
- `Dialog.Title` — header (`<h2>`)
- `Dialog.Description` — description (`<div>`)
- `Dialog.CloseTrigger` — close button (`<button>`)
- `Dialog.RootProvider` — context provider

### 11.2 Root Props

| Prop                     | Type                        | Default    | Description                       |
| ------------------------ | --------------------------- | ---------- | --------------------------------- |
| `aria-label`             | `string`                    | —          | Accessible label if title omitted |
| `closeOnEscape`          | `boolean`                   | `true`     | Close on Escape                   |
| `closeOnInteractOutside` | `boolean`                   | `true`     | Close on outside click            |
| `defaultOpen`            | `boolean`                   | `false`    | Initial open state                |
| `finalFocusEl`           | `() => MaybeElement`        | —          | Focus target on close             |
| `id`                     | `string`                    | —          | Unique identifier                 |
| `initialFocusEl`         | `() => MaybeElement`        | —          | Focus target on open              |
| `lazyMount`              | `boolean`                   | `false`    | Lazy mount content                |
| `modal`                  | `boolean`                   | `true`     | Prevent outside interaction       |
| `open`                   | `boolean`                   | —          | Controlled open state             |
| `preventScroll`          | `boolean`                   | `true`     | Block scrolling                   |
| `restoreFocus`           | `boolean`                   | —          | Restore focus on close            |
| `role`                   | `'dialog' \| 'alertdialog'` | `'dialog'` | ARIA role                         |
| `trapFocus`              | `boolean`                   | `true`     | Trap focus inside                 |
| `unmountOnExit`          | `boolean`                   | `false`    | Remove DOM on close               |

### 11.3 Events

| Event                  | Payload                   | Description         |
| ---------------------- | ------------------------- | ------------------- |
| `onOpenChange`         | `OpenChangeDetails`       | Open state changes  |
| `onEscapeKeyDown`      | `KeyboardEvent`           | Escape pressed      |
| `onInteractOutside`    | `InteractOutsideEvent`    | Outside interaction |
| `onFocusOutside`       | `FocusOutsideEvent`       | Focus moved outside |
| `onPointerDownOutside` | `PointerDownOutsideEvent` | Pointer outside     |
| `onExitComplete`       | —                         | Animation finished  |
| `onRequestDismiss`     | `LayerDismissEvent`       | Parent layer closed |

### 11.4 Data Attributes

| Part             | Attribute           | Values                      |
| ---------------- | ------------------- | --------------------------- |
| Content/Backdrop | `[data-state]`      | `"open" \| "closed"`        |
| Content          | `[data-nested]`     | Present on nested dialogs   |
| Content          | `[data-has-nested]` | Parent with nested children |

### 11.5 CSS Variables

| Variable               | Description      |
| ---------------------- | ---------------- |
| `--layer-index`        | Z-index stacking |
| `--nested-layer-count` | Nested depth     |

---

## 12. Editable

**Ark-UI name:** `Editable`
**Category:** Input

### 12.1 Anatomy (Parts)

- `Editable.Root` — container (`<div>`)
- `Editable.Label` — label (`<label>`)
- `Editable.Area` — area wrapper (`<div>`)
- `Editable.Input` — edit input (`<input>`)
- `Editable.Preview` — preview text (`<span>`)
- `Editable.Control` — controls wrapper (`<div>`)
- `Editable.EditTrigger` — edit button (`<button>`)
- `Editable.SubmitTrigger` — submit button (`<button>`)
- `Editable.CancelTrigger` — cancel button (`<button>`)
- `Editable.RootProvider` — context provider

### 12.2 Root Props

| Prop             | Type                                          | Default   | Description            |
| ---------------- | --------------------------------------------- | --------- | ---------------------- |
| `activationMode` | `'focus' \| 'dblclick' \| 'click' \| 'none'`  | `'focus'` | Edit trigger mode      |
| `autoResize`     | `boolean`                                     | —         | Auto-grow with content |
| `defaultEdit`    | `boolean`                                     | —         | Start in edit mode     |
| `defaultValue`   | `string`                                      | —         | Initial value          |
| `disabled`       | `boolean`                                     | —         | Disabled               |
| `edit`           | `boolean`                                     | —         | Controlled edit state  |
| `finalFocusEl`   | `() => HTMLElement \| null`                   | —         | Focus on close         |
| `form`           | `string`                                      | —         | Form ID                |
| `id`             | `string`                                      | —         | Unique ID              |
| `ids`            | `Partial<{...}>`                              | —         | Custom IDs             |
| `invalid`        | `boolean`                                     | —         | Invalid                |
| `maxLength`      | `number`                                      | —         | Max characters         |
| `name`           | `string`                                      | —         | Form name              |
| `placeholder`    | `string \| { edit: string; preview: string }` | —         | Placeholder text       |
| `readOnly`       | `boolean`                                     | —         | Read-only              |
| `required`       | `boolean`                                     | —         | Required               |
| `selectOnFocus`  | `boolean`                                     | `true`    | Select text on focus   |
| `submitMode`     | `'enter' \| 'blur' \| 'none' \| 'both'`       | `'both'`  | Submit trigger         |
| `value`          | `string`                                      | —         | Controlled value       |

### 12.3 Events

| Event                  | Payload                   | Description          |
| ---------------------- | ------------------------- | -------------------- |
| `onEditChange`         | `EditChangeDetails`       | Edit mode toggle     |
| `onValueChange`        | `ValueChangeDetails`      | Value modified       |
| `onValueCommit`        | `ValueChangeDetails`      | Value submitted      |
| `onValueRevert`        | `ValueChangeDetails`      | Changes discarded    |
| `onFocusOutside`       | `FocusOutsideEvent`       | External focus       |
| `onInteractOutside`    | `InteractOutsideEvent`    | External interaction |
| `onPointerDownOutside` | `PointerDownOutsideEvent` | Pointer outside      |

### 12.4 Data Attributes

| Part  | Attribute                                                                   | Values                 |
| ----- | --------------------------------------------------------------------------- | ---------------------- |
| Area  | `[data-focus]`                                                              | When focused           |
| Area  | `[data-placeholder-shown]`                                                  | When placeholder shown |
| Input | `[data-disabled]`, `[data-readonly]`, `[data-invalid]`, `[data-autoresize]` | State indicators       |

### 12.5 Context API

| Property     | Type                      | Description          |
| ------------ | ------------------------- | -------------------- |
| `editing`    | `boolean`                 | Edit mode state      |
| `empty`      | `boolean`                 | Value empty          |
| `value`      | `string`                  | Current value        |
| `valueText`  | `string`                  | Value or placeholder |
| `setValue`   | `(value: string) => void` | Update value         |
| `clearValue` | `VoidFunction`            | Reset                |
| `edit`       | `VoidFunction`            | Enter edit mode      |
| `cancel`     | `VoidFunction`            | Exit without saving  |
| `submit`     | `VoidFunction`            | Exit with save       |

---

## 13. Field

**Ark-UI name:** `Field`
**Category:** Utility

### 13.1 Anatomy (Parts)

- `Field.Root` — container
- `Field.Label` — label
- `Field.Input` — input
- `Field.Textarea` — textarea
- `Field.Select` — select
- `Field.HelperText` — helper text
- `Field.ErrorText` — error message
- `Field.RequiredIndicator` — required marker
- `Field.RootProvider` — context provider

### 13.2 Root Props

| Prop       | Type         | Default | Description     |
| ---------- | ------------ | ------- | --------------- |
| `disabled` | `boolean`    | —       | Disabled state  |
| `invalid`  | `boolean`    | —       | Invalid state   |
| `readOnly` | `boolean`    | —       | Read-only state |
| `required` | `boolean`    | —       | Required state  |
| `ids`      | `ElementIds` | —       | Custom IDs      |
| `asChild`  | `boolean`    | —       | Composition     |

### 13.3 Textarea Props

| Prop         | Type      | Default | Description |
| ------------ | --------- | ------- | ----------- |
| `autoresize` | `boolean` | `false` | Auto-resize |
| `asChild`    | `boolean` | —       | Composition |

### 13.4 RequiredIndicator Props

| Prop       | Type        | Description      |
| ---------- | ----------- | ---------------- |
| `fallback` | `ReactNode` | Fallback content |
| `asChild`  | `boolean`   | Composition      |

---

## 14. Fieldset

**Ark-UI name:** `Fieldset`
**Category:** Utility

### 14.1 Anatomy (Parts)

- `Fieldset.Root` — fieldset (`<fieldset>`)
- `Fieldset.Legend` — legend (`<legend>`)
- `Fieldset.HelperText` — helper text (`<span>`)
- `Fieldset.ErrorText` — error text (`<span>`)
- `Fieldset.RootProvider` — context provider

### 14.2 Root Props

| Prop      | Type      | Default | Description   |
| --------- | --------- | ------- | ------------- |
| `asChild` | `boolean` | —       | Composition   |
| `invalid` | `boolean` | —       | Invalid state |

---

## 15. File Upload

**Ark-UI name:** `FileUpload`
**Category:** Specialized

### 15.1 Anatomy (Parts)

- `FileUpload.Root`, `.Label`, `.Dropzone`, `.Trigger`
- `FileUpload.ItemGroup`, `.Item`, `.ItemPreview`, `.ItemPreviewImage`
- `FileUpload.ItemName`, `.ItemSizeText`, `.ItemDeleteTrigger`
- `FileUpload.ClearTrigger`, `.HiddenInput`, `.RootProvider`

### 15.2 Root Props

| Prop                   | Type                                                                | Default    | Description                   |
| ---------------------- | ------------------------------------------------------------------- | ---------- | ----------------------------- |
| `accept`               | `Record<string, string[]> \| FileMimeType \| FileMimeType[]`        | —          | Permitted file types          |
| `acceptedFiles`        | `File[]`                                                            | —          | Controlled accepted files     |
| `allowDrop`            | `boolean`                                                           | `true`     | Drag-and-drop toggle          |
| `capture`              | `'user' \| 'environment'`                                           | —          | Camera selection              |
| `defaultAcceptedFiles` | `File[]`                                                            | —          | Initial files                 |
| `directory`            | `boolean`                                                           | —          | Directory upload              |
| `disabled`             | `boolean`                                                           | —          | Disabled                      |
| `invalid`              | `boolean`                                                           | —          | Invalid                       |
| `locale`               | `string`                                                            | `'en-US'`  | Locale                        |
| `maxFiles`             | `number`                                                            | `1`        | Max files                     |
| `maxFileSize`          | `number`                                                            | `Infinity` | Max size (bytes)              |
| `minFileSize`          | `number`                                                            | `0`        | Min size (bytes)              |
| `name`                 | `string`                                                            | —          | Input name                    |
| `preventDocumentDrop`  | `boolean`                                                           | `true`     | Prevent accidental navigation |
| `readOnly`             | `boolean`                                                           | —          | Read-only                     |
| `required`             | `boolean`                                                           | —          | Required                      |
| `transformFiles`       | `(files: File[]) => Promise<File[]>`                                | —          | Transform function            |
| `validate`             | `(file: File, details: FileValidateDetails) => FileError[] \| null` | —          | Custom validation             |

### 15.3 Events

| Event          | Payload             | Description    |
| -------------- | ------------------- | -------------- |
| `onFileAccept` | `FileAcceptDetails` | Files accepted |
| `onFileChange` | `FileChangeDetails` | Files changed  |
| `onFileReject` | `FileRejectDetails` | Files rejected |

### 15.4 Dropzone Props

| Prop           | Type      | Description         |
| -------------- | --------- | ------------------- |
| `disableClick` | `boolean` | Prevent double-open |
| `asChild`      | `boolean` | Composition         |

### 15.5 Item Props

| Prop      | Type      | Description |
| --------- | --------- | ----------- |
| `file`    | `File`    | File object |
| `asChild` | `boolean` | Composition |

### 15.6 ItemPreview Props

| Prop      | Type      | Default | Description      |
| --------- | --------- | ------- | ---------------- |
| `type`    | `string`  | `'.*'`  | MIME type filter |
| `asChild` | `boolean` | —       | Composition      |

### 15.7 Data Attributes

| Part     | Attribute         | Values                 |
| -------- | ----------------- | ---------------------- |
| Root     | `[data-disabled]` | Present when disabled  |
| Root     | `[data-readonly]` | Present when read-only |
| Root     | `[data-dragging]` | Present during drag    |
| Dropzone | `[data-invalid]`  | Present when invalid   |
| Dropzone | `[data-dragging]` | Present during drag    |

### 15.8 Context API

| Property             | Type                                                      | Description        |
| -------------------- | --------------------------------------------------------- | ------------------ |
| `dragging`           | `boolean`                                                 | Drag state         |
| `focused`            | `boolean`                                                 | Focus state        |
| `disabled`           | `boolean`                                                 | Disabled           |
| `maxFilesReached`    | `boolean`                                                 | At limit           |
| `remainingFiles`     | `number`                                                  | Remaining capacity |
| `acceptedFiles`      | `File[]`                                                  | Accepted files     |
| `rejectedFiles`      | `FileRejection[]`                                         | Rejected files     |
| `openFilePicker`     | `VoidFunction`                                            | Open picker        |
| `deleteFile`         | `(file: File) => void`                                    | Remove file        |
| `setFiles`           | `(files: File[]) => void`                                 | Set files          |
| `clearFiles`         | `VoidFunction`                                            | Clear all          |
| `clearRejectedFiles` | `VoidFunction`                                            | Clear rejected     |
| `getFileSize`        | `(file: File) => string`                                  | Get formatted size |
| `createFileUrl`      | `(file: File, cb: (url: string) => void) => VoidFunction` | Create preview URL |

---

## 16. Floating Panel

**Ark-UI name:** `FloatingPanel`
**Category:** Overlay

### 16.1 Anatomy (Parts)

- `FloatingPanel.Root`, `.Trigger`, `.Positioner`, `.Content`
- `FloatingPanel.DragTrigger`, `.Header`, `.Title`
- `FloatingPanel.Control`, `.StageTrigger`, `.CloseTrigger`
- `FloatingPanel.Body`, `.ResizeTrigger`, `.RootProvider`

### 16.2 Root Props

| Prop                | Type                        | Default   | Description                 |
| ------------------- | --------------------------- | --------- | --------------------------- |
| `allowOverflow`     | `boolean`                   | `true`    | Allow overflow during drag  |
| `closeOnEscape`     | `boolean`                   | —         | Close on Escape             |
| `defaultOpen`       | `boolean`                   | `false`   | Initial open                |
| `defaultPosition`   | `Point`                     | —         | Initial position            |
| `defaultSize`       | `Size`                      | —         | Default dimensions          |
| `dir`               | `'ltr' \| 'rtl'`            | `'ltr'`   | Text direction              |
| `disabled`          | `boolean`                   | —         | Disabled                    |
| `draggable`         | `boolean`                   | `true`    | Enable dragging             |
| `getAnchorPosition` | `(details) => Point`        | —         | Initial position calculator |
| `getBoundaryEl`     | `() => HTMLElement \| null` | —         | Boundary element            |
| `gridSize`          | `number`                    | `1`       | Snap grid                   |
| `id`                | `string`                    | —         | Unique ID                   |
| `lazyMount`         | `boolean`                   | `false`   | Lazy mounting               |
| `lockAspectRatio`   | `boolean`                   | —         | Lock aspect ratio           |
| `maxSize`           | `Size`                      | —         | Max dimensions              |
| `minSize`           | `Size`                      | —         | Min dimensions              |
| `open`              | `boolean`                   | —         | Controlled open             |
| `persistRect`       | `boolean`                   | —         | Preserve position on close  |
| `position`          | `Point`                     | —         | Controlled position         |
| `resizable`         | `boolean`                   | `true`    | Enable resizing             |
| `size`              | `Size`                      | —         | Controlled size             |
| `strategy`          | `'absolute' \| 'fixed'`     | `'fixed'` | Positioning strategy        |

### 16.3 Events

| Event                 | Payload                 | Description        |
| --------------------- | ----------------------- | ------------------ |
| `onOpenChange`        | `OpenChangeDetails`     | Open state change  |
| `onPositionChange`    | `PositionChangeDetails` | Position change    |
| `onPositionChangeEnd` | `PositionChangeDetails` | Drag end           |
| `onSizeChange`        | `SizeChangeDetails`     | Size change        |
| `onStageChange`       | `StageChangeDetails`    | Stage transition   |
| `onExitComplete`      | —                       | Animation complete |

### 16.4 Data Attributes

| Attribute          | Values                   |
| ------------------ | ------------------------ |
| `[data-dragging]`  | Present during drag      |
| `[data-minimized]` | Present when minimized   |
| `[data-maximized]` | Present when maximized   |
| `[data-staged]`    | Present when not default |
| `[data-state]`     | `"open" \| "closed"`     |
| `[data-topmost]`   | Highest z-index          |
| `[data-behind]`    | Not topmost              |

### 16.5 Context API

| Property      | Type                      | Description      |
| ------------- | ------------------------- | ---------------- |
| `open`        | `boolean`                 | Open state       |
| `setOpen`     | `(open: boolean) => void` | Toggle           |
| `position`    | `Point`                   | Current position |
| `setPosition` | `(pos: Point) => void`    | Update position  |
| `size`        | `Size`                    | Current size     |
| `setSize`     | `(size: Size) => void`    | Update size      |
| `minimize`    | `VoidFunction`            | Minimize         |
| `maximize`    | `VoidFunction`            | Maximize         |
| `restore`     | `VoidFunction`            | Restore          |
| `dragging`    | `boolean`                 | Drag state       |
| `resizing`    | `boolean`                 | Resize state     |

---

## 17. Hover Card

**Ark-UI name:** `HoverCard`
**Category:** Overlay

### 17.1 Anatomy (Parts)

- `HoverCard.Root`, `.Trigger`, `.Positioner`, `.Content`
- `HoverCard.Arrow`, `.ArrowTip`, `.RootProvider`

### 17.2 Root Props

| Prop                   | Type                                             | Default | Description            |
| ---------------------- | ------------------------------------------------ | ------- | ---------------------- |
| `closeDelay`           | `number`                                         | `300`   | Close delay (ms)       |
| `defaultOpen`          | `boolean`                                        | —       | Initial open           |
| `disabled`             | `boolean`                                        | —       | Disabled               |
| `id`                   | `string`                                         | —       | Unique ID              |
| `ids`                  | `Partial<{trigger, content, positioner, arrow}>` | —       | Custom IDs             |
| `immediate`            | `boolean`                                        | —       | Sync immediately       |
| `lazyMount`            | `boolean`                                        | `false` | Lazy mount             |
| `open`                 | `boolean`                                        | —       | Controlled open        |
| `openDelay`            | `number`                                         | `600`   | Open delay (ms)        |
| `positioning`          | `PositioningOptions`                             | —       | Positioning            |
| `skipAnimationOnMount` | `boolean`                                        | `false` | Skip initial animation |
| `unmountOnExit`        | `boolean`                                        | `false` | Unmount on exit        |

### 17.3 Events

| Event                  | Payload                   | Description         |
| ---------------------- | ------------------------- | ------------------- |
| `onOpenChange`         | `OpenChangeDetails`       | Open state change   |
| `onFocusOutside`       | `FocusOutsideEvent`       | Focus outside       |
| `onInteractOutside`    | `InteractOutsideEvent`    | Interaction outside |
| `onPointerDownOutside` | `PointerDownOutsideEvent` | Pointer outside     |
| `onExitComplete`       | —                         | Animation complete  |

### 17.4 Data Attributes

| Part            | Attribute          | Values               |
| --------------- | ------------------ | -------------------- |
| Trigger/Content | `[data-state]`     | `"open" \| "closed"` |
| Content         | `[data-placement]` | Placement direction  |

### 17.5 Context API

| Property     | Type                      | Description   |
| ------------ | ------------------------- | ------------- |
| `open`       | `boolean`                 | Open state    |
| `setOpen`    | `(open: boolean) => void` | Control state |
| `reposition` | `(options?) => void`      | Reposition    |

---

## 18. Image Cropper

**Ark-UI name:** `ImageCropper`
**Category:** Specialized

### 18.1 Anatomy (Parts)

- `ImageCropper.Root`, `.Viewport`, `.Image`
- `ImageCropper.Selection`, `.Handle`, `.Grid`
- `ImageCropper.RootProvider`

### 18.2 Root Props

| Prop              | Type                      | Default                                  | Description            |
| ----------------- | ------------------------- | ---------------------------------------- | ---------------------- |
| `aspectRatio`     | `number`                  | —                                        | Crop area aspect ratio |
| `cropShape`       | `'circle' \| 'rectangle'` | `'rectangle'`                            | Crop shape             |
| `defaultFlip`     | `FlipState`               | `{ horizontal: false, vertical: false }` | Initial flip           |
| `defaultRotation` | `number`                  | `0`                                      | Initial rotation       |
| `defaultZoom`     | `number`                  | `1`                                      | Initial zoom           |
| `fixedCropArea`   | `boolean`                 | `false`                                  | Fixed crop area        |
| `flip`            | `FlipState`               | —                                        | Controlled flip        |
| `initialCrop`     | `Rect`                    | —                                        | Initial crop area      |
| `maxHeight`       | `number`                  | `Infinity`                               | Max crop height        |
| `maxWidth`        | `number`                  | `Infinity`                               | Max crop width         |
| `maxZoom`         | `number`                  | `5`                                      | Max zoom               |
| `minHeight`       | `number`                  | `40`                                     | Min crop height        |
| `minWidth`        | `number`                  | `40`                                     | Min crop width         |
| `minZoom`         | `number`                  | `1`                                      | Min zoom               |
| `nudgeStep`       | `number`                  | `1`                                      | Arrow key step (px)    |
| `nudgeStepCtrl`   | `number`                  | `50`                                     | Ctrl+Arrow step        |
| `nudgeStepShift`  | `number`                  | `10`                                     | Shift+Arrow step       |
| `rotation`        | `number`                  | —                                        | Controlled rotation    |
| `translations`    | `IntlTranslations`        | —                                        | i18n strings           |
| `zoom`            | `number`                  | —                                        | Controlled zoom        |
| `zoomSensitivity` | `number`                  | `2`                                      | Pinch sensitivity      |
| `zoomStep`        | `number`                  | `0.1`                                    | Wheel zoom step        |

### 18.3 Events

| Event              | Payload                 | Description       |
| ------------------ | ----------------------- | ----------------- |
| `onCropChange`     | `CropChangeDetails`     | Crop area changed |
| `onFlipChange`     | `FlipChangeDetails`     | Flip changed      |
| `onRotationChange` | `RotationChangeDetails` | Rotation changed  |
| `onZoomChange`     | `ZoomChangeDetails`     | Zoom changed      |

### 18.4 Handle Props

| Prop       | Type             | Description                |
| ---------- | ---------------- | -------------------------- |
| `position` | `HandlePosition` | Handle position (required) |
| `asChild`  | `boolean`        | Composition                |

### 18.5 Grid Props

| Prop      | Type                         | Description |
| --------- | ---------------------------- | ----------- |
| `axis`    | `'horizontal' \| 'vertical'` | Grid axis   |
| `asChild` | `boolean`                    | Composition |

### 18.6 CSS Variables

| Variable                               | Description     |
| -------------------------------------- | --------------- |
| `--crop-width`, `--crop-height`        | Crop dimensions |
| `--crop-x`, `--crop-y`                 | Crop position   |
| `--image-zoom`                         | Zoom value      |
| `--image-rotation`                     | Rotation value  |
| `--image-offset-x`, `--image-offset-y` | Pan offset      |

### 18.7 Context API

| Property           | Type                                    | Description       |
| ------------------ | --------------------------------------- | ----------------- |
| `zoom`             | `number`                                | Current zoom      |
| `rotation`         | `number`                                | Current rotation  |
| `flip`             | `FlipState`                             | Current flip      |
| `crop`             | `Rect`                                  | Current crop      |
| `offset`           | `Point`                                 | Pan position      |
| `dragging`         | `boolean`                               | Dragging state    |
| `panning`          | `boolean`                               | Panning state     |
| `setZoom`          | `(zoom: number) => void`                | Set zoom          |
| `zoomBy`           | `(delta: number) => void`               | Relative zoom     |
| `setRotation`      | `(rotation: number) => void`            | Set rotation      |
| `rotateBy`         | `(degrees: number) => void`             | Relative rotation |
| `setFlip`          | `(flip: Partial<FlipState>) => void`    | Update flip       |
| `flipHorizontally` | `(value?: boolean) => void`             | Horizontal flip   |
| `flipVertically`   | `(value?: boolean) => void`             | Vertical flip     |
| `reset`            | `VoidFunction`                          | Reset all         |
| `getCroppedImage`  | `(options?) => Promise<string \| Blob>` | Get cropped image |
| `getCropData`      | `() => CropData`                        | Get crop data     |

---

## 19. Listbox

**Ark-UI name:** `Listbox`
**Category:** Selection

### 19.1 Anatomy (Parts)

- `Listbox.Root`, `.Label`, `.Content`
- `Listbox.ItemGroup`, `.ItemGroupLabel`
- `Listbox.Item`, `.ItemText`, `.ItemIndicator`
- `Listbox.Empty`, `.Input`, `.ValueText`
- `Listbox.RootProvider`

### 19.2 Root Props

| Prop                      | Type                                   | Default      | Description              |
| ------------------------- | -------------------------------------- | ------------ | ------------------------ |
| `collection`              | `ListCollection<T>`                    | —            | Item collection          |
| `value`                   | `string[]`                             | —            | Controlled selection     |
| `defaultValue`            | `string[]`                             | `[]`         | Initial selection        |
| `selectionMode`           | `'single' \| 'multiple' \| 'extended'` | `'single'`   | Selection mode           |
| `disabled`                | `boolean`                              | —            | Disabled                 |
| `deselectable`            | `boolean`                              | —            | Allow deselect all       |
| `orientation`             | `'horizontal' \| 'vertical'`           | `'vertical'` | Layout                   |
| `loopFocus`               | `boolean`                              | `false`      | Loop navigation          |
| `typeahead`               | `boolean`                              | —            | Typeahead                |
| `selectOnHighlight`       | `boolean`                              | —            | Auto-select on highlight |
| `disallowSelectAll`       | `boolean`                              | —            | Prevent Ctrl+A           |
| `highlightedValue`        | `string`                               | —            | Controlled highlight     |
| `defaultHighlightedValue` | `string`                               | —            | Initial highlight        |
| `scrollToIndexFn`         | `(details) => void`                    | —            | Scroll function          |

### 19.3 Events

| Event               | Payload                     | Description       |
| ------------------- | --------------------------- | ----------------- |
| `onValueChange`     | `ValueChangeDetails<T>`     | Selection changed |
| `onHighlightChange` | `HighlightChangeDetails<T>` | Highlight changed |
| `onSelect`          | `SelectionDetails`          | Item selected     |

### 19.4 Item Props

| Prop               | Type      | Description        |
| ------------------ | --------- | ------------------ |
| `item`             | `any`     | Item data          |
| `highlightOnHover` | `boolean` | Highlight on hover |
| `asChild`          | `boolean` | Composition        |

### 19.5 Input Props

| Prop            | Type      | Default | Description              |
| --------------- | --------- | ------- | ------------------------ |
| `autoHighlight` | `boolean` | `false` | Auto-highlight on typing |
| `asChild`       | `boolean` | —       | Composition              |

### 19.6 ValueText Props

| Prop          | Type      | Description     |
| ------------- | --------- | --------------- |
| `placeholder` | `string`  | Text when empty |
| `asChild`     | `boolean` | Composition     |

### 19.7 Data Attributes

| Attribute                 | Values                     |
| ------------------------- | -------------------------- |
| `[data-activedescendant]` | Active item                |
| `[data-orientation]`      | Layout direction           |
| `[data-empty]`            | No items                   |
| `[data-value]`            | Item value                 |
| `[data-selected]`         | Item selected              |
| `[data-highlighted]`      | Item highlighted           |
| `[data-state]`            | `"checked" \| "unchecked"` |

### 19.8 Context API

| Property           | Type                        | Description     |
| ------------------ | --------------------------- | --------------- |
| `selectedItems`    | `V[]`                       | Selected items  |
| `value`            | `string[]`                  | Selected keys   |
| `highlightedValue` | `string`                    | Highlighted key |
| `empty`            | `boolean`                   | No selection    |
| `selectValue`      | `(value: string) => void`   | Select item     |
| `setValue`         | `(value: string[]) => void` | Set values      |
| `clearValue`       | `(value?: string) => void`  | Clear           |
| `selectAll`        | `VoidFunction`              | Select all      |
| `collection`       | `ListCollection<V>`         | Collection      |

---

## 20. Marquee

**Ark-UI name:** `Marquee`
**Category:** Data Display

### 20.1 Anatomy (Parts)

- `Marquee.Root`, `.Viewport`, `.Content`, `.Item`, `.Edge`, `.RootProvider`

### 20.2 Root Props

| Prop                 | Type               | Default   | Description               |
| -------------------- | ------------------ | --------- | ------------------------- |
| `autoFill`           | `boolean`          | `false`   | Auto-duplicate content    |
| `defaultPaused`      | `boolean`          | `false`   | Initial paused            |
| `delay`              | `number`           | `0`       | Animation delay (seconds) |
| `loopCount`          | `number`           | `0`       | Loop count (0=infinite)   |
| `pauseOnInteraction` | `boolean`          | `false`   | Pause on hover/focus      |
| `reverse`            | `boolean`          | `false`   | Reverse direction         |
| `side`               | `Side`             | `'start'` | Scroll direction          |
| `spacing`            | `string`           | `'1rem'`  | Content gap               |
| `speed`              | `number`           | `50`      | Speed (px/second)         |
| `paused`             | `boolean`          | —         | Controlled pause          |
| `translations`       | `IntlTranslations` | —         | i18n messages             |

### 20.3 Events

| Event            | Payload              | Description           |
| ---------------- | -------------------- | --------------------- |
| `onComplete`     | —                    | Finite loops finished |
| `onLoopComplete` | —                    | Each loop iteration   |
| `onPauseChange`  | `PauseChangeDetails` | Pause state change    |

### 20.4 Edge Props

| Prop   | Type   | Description       |
| ------ | ------ | ----------------- |
| `side` | `Side` | Gradient location |

### 20.5 CSS Variables

| Variable               | Description        |
| ---------------------- | ------------------ |
| `--marquee-duration`   | Animation duration |
| `--marquee-spacing`    | Content gap        |
| `--marquee-delay`      | Start delay        |
| `--marquee-loop-count` | Loop iterations    |
| `--marquee-translate`  | Transform distance |

---

## 21. Menu

**Ark-UI name:** `Menu`
**Category:** Selection

### 21.1 Anatomy (Parts)

- `Menu.Root`, `.Trigger`, `.ContextTrigger`
- `Menu.Positioner`, `.Content`, `.Arrow`, `.ArrowTip`
- `Menu.Item`, `.ItemGroup`, `.ItemGroupLabel`, `.ItemText`, `.ItemIndicator`
- `Menu.CheckboxItem`, `.RadioItemGroup`, `.RadioItem`
- `Menu.Separator`, `.TriggerItem`, `.Indicator`
- `Menu.RootProvider`

### 21.2 Root Props

| Prop                      | Type                 | Default | Description          |
| ------------------------- | -------------------- | ------- | -------------------- |
| `anchorPoint`             | `Point`              | —       | Position reference   |
| `aria-label`              | `string`             | —       | Accessibility label  |
| `closeOnSelect`           | `boolean`            | `true`  | Auto-close on select |
| `composite`               | `boolean`            | `true`  | Composed widget      |
| `defaultHighlightedValue` | `string`             | —       | Initial highlight    |
| `defaultOpen`             | `boolean`            | —       | Initial open         |
| `highlightedValue`        | `string`             | —       | Controlled highlight |
| `id`                      | `string`             | —       | Unique ID            |
| `ids`                     | `Partial<{...}>`     | —       | Custom IDs           |
| `loopFocus`               | `boolean`            | `false` | Loop navigation      |
| `lazyMount`               | `boolean`            | `false` | Lazy mount           |
| `open`                    | `boolean`            | —       | Controlled open      |
| `positioning`             | `PositioningOptions` | —       | Position config      |
| `typeahead`               | `boolean`            | `true`  | Typeahead nav        |
| `unmountOnExit`           | `boolean`            | `false` | Unmount on close     |

### 21.3 Events

| Event                  | Payload                   | Description         |
| ---------------------- | ------------------------- | ------------------- |
| `onSelect`             | `SelectionDetails`        | Item selected       |
| `onOpenChange`         | `OpenChangeDetails`       | Open state change   |
| `onHighlightChange`    | `HighlightChangeDetails`  | Highlight change    |
| `onEscapeKeyDown`      | `KeyboardEvent`           | Escape pressed      |
| `onInteractOutside`    | `InteractOutsideEvent`    | Outside interaction |
| `onFocusOutside`       | `FocusOutsideEvent`       | Outside focus       |
| `onPointerDownOutside` | `PointerDownOutsideEvent` | Pointer outside     |
| `onExitComplete`       | —                         | Animation complete  |
| `onRequestDismiss`     | `LayerDismissEvent`       | Dismiss request     |

### 21.4 Data Attributes

| Part    | Attribute            | Values                |
| ------- | -------------------- | --------------------- |
| Content | `[data-state]`       | `"open" \| "closed"`  |
| Content | `[data-nested]`      | Present for submenus  |
| Content | `[data-has-nested]`  | Has submenus          |
| Content | `[data-placement]`   | Position direction    |
| Item    | `[data-value]`       | Item identifier       |
| Item    | `[data-valuetext]`   | Human-readable value  |
| Item    | `[data-disabled]`    | Present when disabled |
| Item    | `[data-highlighted]` | Present when focused  |
| Trigger | `[data-state]`       | `"open" \| "closed"`  |

### 21.5 Context API

| Property              | Type                      | Description       |
| --------------------- | ------------------------- | ----------------- |
| `open`                | `boolean`                 | Open state        |
| `setOpen`             | `(open: boolean) => void` | Toggle            |
| `highlightedValue`    | `string`                  | Current highlight |
| `setHighlightedValue` | `(value: string) => void` | Set highlight     |
| `reposition`          | `(options?) => void`      | Reposition        |
| `getItemState`        | `(props) => ItemState`    | Get state         |

---

## 22. Number Input

**Ark-UI name:** `NumberInput`
**Category:** Input

### 22.1 Anatomy (Parts)

- `NumberInput.Root`, `.Label`, `.Scrubber`
- `NumberInput.Control`, `.Input`
- `NumberInput.IncrementTrigger`, `.DecrementTrigger`
- `NumberInput.ValueText`, `.RootProvider`

### 22.2 Root Props

| Prop                 | Type                  | Default                   | Description               |
| -------------------- | --------------------- | ------------------------- | ------------------------- |
| `allowMouseWheel`    | `boolean`             | —                         | Mouse wheel changes value |
| `allowOverflow`      | `boolean`             | `true`                    | Allow overflow min/max    |
| `clampValueOnBlur`   | `boolean`             | `true`                    | Clamp on blur             |
| `defaultValue`       | `string`              | —                         | Initial value             |
| `disabled`           | `boolean`             | —                         | Disabled                  |
| `focusInputOnChange` | `boolean`             | `true`                    | Focus on change           |
| `form`               | `string`              | —                         | Form ID                   |
| `formatOptions`      | `NumberFormatOptions` | —                         | Intl.NumberFormat options |
| `id`                 | `string`              | —                         | Unique ID                 |
| `ids`                | `Partial<{...}>`      | —                         | Custom IDs                |
| `inputMode`          | `InputMode`           | `'decimal'`               | Input mode hint           |
| `invalid`            | `boolean`             | —                         | Invalid                   |
| `locale`             | `string`              | `'en-US'`                 | Locale                    |
| `max`                | `number`              | `Number.MAX_SAFE_INTEGER` | Maximum                   |
| `min`                | `number`              | `Number.MIN_SAFE_INTEGER` | Minimum                   |
| `name`               | `string`              | —                         | Form name                 |
| `pattern`            | `string`              | `'-?[0-9]*(.[0-9]+)?'`    | Validation pattern        |
| `readOnly`           | `boolean`             | —                         | Read-only                 |
| `required`           | `boolean`             | —                         | Required                  |
| `spinOnPress`        | `boolean`             | `true`                    | Spin on hold              |
| `step`               | `number`              | `1`                       | Step amount               |
| `translations`       | `IntlTranslations`    | —                         | i18n                      |
| `value`              | `string`              | —                         | Controlled value          |

### 22.3 Events

| Event            | Payload               | Description     |
| ---------------- | --------------------- | --------------- |
| `onFocusChange`  | `FocusChangeDetails`  | Focus change    |
| `onValueChange`  | `ValueChangeDetails`  | Value change    |
| `onValueCommit`  | `ValueChangeDetails`  | Value committed |
| `onValueInvalid` | `ValueInvalidDetails` | Over/underflow  |

### 22.4 Context API

| Property        | Type                      | Description     |
| --------------- | ------------------------- | --------------- |
| `focused`       | `boolean`                 | Focus state     |
| `invalid`       | `boolean`                 | Invalid         |
| `empty`         | `boolean`                 | Empty           |
| `value`         | `string`                  | Formatted value |
| `valueAsNumber` | `number`                  | Numeric value   |
| `setValue`      | `(value: number) => void` | Set value       |
| `clearValue`    | `VoidFunction`            | Clear           |
| `increment`     | `VoidFunction`            | Step up         |
| `decrement`     | `VoidFunction`            | Step down       |
| `setToMax`      | `VoidFunction`            | Set to max      |
| `setToMin`      | `VoidFunction`            | Set to min      |
| `focus`         | `VoidFunction`            | Focus input     |

---

## 23. Pagination

**Ark-UI name:** `Pagination`
**Category:** Navigation

### 23.1 Anatomy (Parts)

- `Pagination.Root`, `.PrevTrigger`, `.NextTrigger`
- `Pagination.FirstTrigger`, `.LastTrigger`
- `Pagination.Item`, `.Ellipsis`, `.RootProvider`

### 23.2 Root Props

| Prop              | Type                  | Default    | Description         |
| ----------------- | --------------------- | ---------- | ------------------- |
| `boundaryCount`   | `number`              | `1`        | Pages at start/end  |
| `count`           | `number`              | —          | Total items         |
| `defaultPage`     | `number`              | `1`        | Initial page        |
| `defaultPageSize` | `number`              | `10`       | Initial page size   |
| `getPageUrl`      | `(details) => string` | —          | URL generator       |
| `page`            | `number`              | —          | Controlled page     |
| `pageSize`        | `number`              | —          | Controlled size     |
| `siblingCount`    | `number`              | `1`        | Pages beside active |
| `translations`    | `IntlTranslations`    | —          | i18n                |
| `type`            | `'button' \| 'link'`  | `'button'` | Element type        |

### 23.3 Events

| Event              | Payload                 | Description       |
| ------------------ | ----------------------- | ----------------- |
| `onPageChange`     | `PageChangeDetails`     | Page changed      |
| `onPageSizeChange` | `PageSizeChangeDetails` | Page size changed |

### 23.4 Item Props

| Prop    | Type     | Description |
| ------- | -------- | ----------- |
| `type`  | `'page'` | Item type   |
| `value` | `number` | Page number |

### 23.5 Ellipsis Props

| Prop    | Type     | Description    |
| ------- | -------- | -------------- |
| `index` | `number` | Ellipsis index |

### 23.6 Context API

| Property        | Type                     | Description     |
| --------------- | ------------------------ | --------------- |
| `page`          | `number`                 | Current page    |
| `totalPages`    | `number`                 | Total pages     |
| `pages`         | `Pages`                  | Page array      |
| `previousPage`  | `number`                 | Previous page   |
| `nextPage`      | `number`                 | Next page       |
| `pageRange`     | `PageRange`              | Start/end range |
| `slice`         | `<V>(data: V[]) => V[]`  | Slice data      |
| `setPageSize`   | `(size: number) => void` | Update size     |
| `setPage`       | `(page: number) => void` | Set page        |
| `goToNextPage`  | `VoidFunction`           | Next            |
| `goToPrevPage`  | `VoidFunction`           | Previous        |
| `goToFirstPage` | `VoidFunction`           | First           |
| `goToLastPage`  | `VoidFunction`           | Last            |

---

## 24. Password Input

**Ark-UI name:** `PasswordInput`
**Category:** Input

### 24.1 Anatomy (Parts)

- `PasswordInput.Root`, `.Label`, `.Control`
- `PasswordInput.Input`, `.VisibilityTrigger`, `.Indicator`
- `PasswordInput.RootProvider`

### 24.2 Root Props

| Prop                     | Type                                   | Default              | Description             |
| ------------------------ | -------------------------------------- | -------------------- | ----------------------- |
| `autoComplete`           | `'current-password' \| 'new-password'` | `'current-password'` | Autocomplete            |
| `defaultVisible`         | `boolean`                              | —                    | Initial visibility      |
| `disabled`               | `boolean`                              | —                    | Disabled                |
| `id`                     | `string`                               | —                    | Unique ID               |
| `ids`                    | `Partial<{input, visibilityTrigger}>`  | —                    | Custom IDs              |
| `ignorePasswordManagers` | `boolean`                              | —                    | Block password managers |
| `invalid`                | `boolean`                              | —                    | Invalid                 |
| `name`                   | `string`                               | —                    | Input name              |
| `readOnly`               | `boolean`                              | —                    | Read-only               |
| `required`               | `boolean`                              | —                    | Required                |
| `translations`           | `Partial<{visibilityTrigger: ...}>`    | —                    | i18n                    |
| `visible`                | `boolean`                              | —                    | Controlled visibility   |

### 24.3 Events

| Event                | Payload                   | Description        |
| -------------------- | ------------------------- | ------------------ |
| `onVisibilityChange` | `VisibilityChangeDetails` | Visibility changed |

### 24.4 Indicator Props

| Prop       | Type           | Description         |
| ---------- | -------------- | ------------------- |
| `fallback` | `ReactElement` | Content when hidden |
| `asChild`  | `boolean`      | Composition         |

### 24.5 Data Attributes

| Part            | Attribute                                              | Values                  |
| --------------- | ------------------------------------------------------ | ----------------------- |
| Input/Indicator | `[data-state]`                                         | `"visible" \| "hidden"` |
| Label           | `[data-required]`                                      | Present when required   |
| All             | `[data-disabled]`, `[data-invalid]`, `[data-readonly]` | State indicators        |

### 24.6 Context API

| Property        | Type                       | Description       |
| --------------- | -------------------------- | ----------------- |
| `visible`       | `boolean`                  | Visibility state  |
| `disabled`      | `boolean`                  | Disabled          |
| `invalid`       | `boolean`                  | Invalid           |
| `focus`         | `VoidFunction`             | Focus input       |
| `setVisible`    | `(value: boolean) => void` | Set visibility    |
| `toggleVisible` | `VoidFunction`             | Toggle visibility |

---

## 25. Pin Input

**Ark-UI name:** `PinInput`
**Category:** Input

### 25.1 Anatomy (Parts)

- `PinInput.Root`, `.Label`, `.Control`, `.Input`, `.HiddenInput`, `.RootProvider`

### 25.2 Root Props

| Prop             | Type                                          | Default     | Description            |
| ---------------- | --------------------------------------------- | ----------- | ---------------------- |
| `autoFocus`      | `boolean`                                     | —           | Auto-focus first input |
| `blurOnComplete` | `boolean`                                     | —           | Blur on complete       |
| `count`          | `number`                                      | —           | Input count for SSR    |
| `defaultValue`   | `string[]`                                    | —           | Initial value          |
| `disabled`       | `boolean`                                     | —           | Disabled               |
| `form`           | `string`                                      | —           | Form ID                |
| `id`             | `string`                                      | —           | Unique ID              |
| `ids`            | `Partial<{...}>`                              | —           | Custom IDs             |
| `invalid`        | `boolean`                                     | —           | Invalid                |
| `mask`           | `boolean`                                     | —           | Mask values            |
| `name`           | `string`                                      | —           | Form name              |
| `otp`            | `boolean`                                     | —           | OTP autocomplete       |
| `pattern`        | `string`                                      | —           | Validation pattern     |
| `placeholder`    | `string`                                      | `'○'`       | Placeholder char       |
| `readOnly`       | `boolean`                                     | —           | Read-only              |
| `required`       | `boolean`                                     | —           | Required               |
| `selectOnFocus`  | `boolean`                                     | —           | Select on focus        |
| `translations`   | `IntlTranslations`                            | —           | i18n                   |
| `type`           | `'numeric' \| 'alphanumeric' \| 'alphabetic'` | `'numeric'` | Value type             |
| `value`          | `string[]`                                    | —           | Controlled value       |

### 25.3 Events

| Event             | Payload               | Description       |
| ----------------- | --------------------- | ----------------- |
| `onValueChange`   | `ValueChangeDetails`  | Value changed     |
| `onValueComplete` | `ValueChangeDetails`  | All fields filled |
| `onValueInvalid`  | `ValueInvalidDetails` | Invalid input     |

### 25.4 Input Props

| Prop    | Type     | Description    |
| ------- | -------- | -------------- |
| `index` | `number` | Input position |

### 25.5 Data Attributes

| Part  | Attribute         | Values                |
| ----- | ----------------- | --------------------- |
| Root  | `[data-complete]` | Present when complete |
| Input | `[data-complete]` | Field filled          |
| Input | `[data-filled]`   | Has value             |
| Input | `[data-index]`    | Position index        |

### 25.6 Context API

| Property          | Type                                     | Description     |
| ----------------- | ---------------------------------------- | --------------- |
| `value`           | `string[]`                               | Current values  |
| `valueAsString`   | `string`                                 | Combined string |
| `complete`        | `boolean`                                | All filled      |
| `count`           | `number`                                 | Total inputs    |
| `setValue`        | `(value: string[]) => void`              | Update all      |
| `clearValue`      | `VoidFunction`                           | Reset all       |
| `setValueAtIndex` | `(index: number, value: string) => void` | Update specific |
| `focus`           | `VoidFunction`                           | Focus first     |

---

## 26. Popover

**Ark-UI name:** `Popover`
**Category:** Overlay

### 26.1 Anatomy (Parts)

- `Popover.Root`, `.Trigger`, `.Anchor`
- `Popover.Positioner`, `.Arrow`, `.ArrowTip`
- `Popover.Content`, `.Title`, `.Description`
- `Popover.CloseTrigger`, `.Indicator`, `.RootProvider`

### 26.2 Root Props

| Prop                     | Type                        | Default | Description                  |
| ------------------------ | --------------------------- | ------- | ---------------------------- |
| `autoFocus`              | `boolean`                   | `true`  | Focus first element on open  |
| `closeOnEscape`          | `boolean`                   | `true`  | Close on Escape              |
| `closeOnInteractOutside` | `boolean`                   | `true`  | Close on outside click       |
| `defaultOpen`            | `boolean`                   | —       | Initial open                 |
| `id`                     | `string`                    | —       | Unique ID                    |
| `ids`                    | `Partial<{...}>`            | —       | Custom IDs                   |
| `immediate`              | `boolean`                   | —       | Sync immediately             |
| `initialFocusEl`         | `() => HTMLElement \| null` | —       | Initial focus element        |
| `lazyMount`              | `boolean`                   | `false` | Lazy mount                   |
| `modal`                  | `boolean`                   | `false` | Modal mode                   |
| `open`                   | `boolean`                   | —       | Controlled open              |
| `persistentElements`     | `(() => Element \| null)[]` | —       | Elements to keep interactive |
| `portalled`              | `boolean`                   | `true`  | Portal content               |
| `positioning`            | `PositioningOptions`        | —       | Position config              |
| `skipAnimationOnMount`   | `boolean`                   | `false` | Skip initial animation       |
| `unmountOnExit`          | `boolean`                   | `false` | Unmount on exit              |

### 26.3 Events

| Event                  | Payload                   | Description         |
| ---------------------- | ------------------------- | ------------------- |
| `onOpenChange`         | `OpenChangeDetails`       | Open state change   |
| `onEscapeKeyDown`      | `KeyboardEvent`           | Escape pressed      |
| `onInteractOutside`    | `InteractOutsideEvent`    | Outside interaction |
| `onFocusOutside`       | `FocusOutsideEvent`       | Focus outside       |
| `onPointerDownOutside` | `PointerDownOutsideEvent` | Pointer outside     |
| `onRequestDismiss`     | `LayerDismissEvent`       | Dismiss request     |
| `onExitComplete`       | —                         | Animation complete  |

### 26.4 Data Attributes

| Part      | Attribute          | Values                |
| --------- | ------------------ | --------------------- |
| Trigger   | `[data-state]`     | `"open" \| "closed"`  |
| Content   | `[data-state]`     | `"open" \| "closed"`  |
| Content   | `[data-expanded]`  | Present when expanded |
| Content   | `[data-placement]` | Position direction    |
| Indicator | `[data-state]`     | `"open" \| "closed"`  |

### 26.5 CSS Variables (Positioner)

| Variable             | Description        |
| -------------------- | ------------------ |
| `--reference-width`  | Reference width    |
| `--reference-height` | Reference height   |
| `--available-width`  | Viewport width     |
| `--available-height` | Viewport height    |
| `--x`, `--y`         | Transform position |
| `--z-index`          | Z-index            |
| `--transform-origin` | Animation origin   |

### 26.6 CSS Variables (Arrow)

| Variable             | Description      |
| -------------------- | ---------------- |
| `--arrow-size`       | Arrow dimensions |
| `--arrow-size-half`  | Half arrow size  |
| `--arrow-background` | Arrow color      |
| `--arrow-offset`     | Arrow offset     |

### 26.7 Context API

| Property     | Type                      | Description  |
| ------------ | ------------------------- | ------------ |
| `portalled`  | `boolean`                 | Portal state |
| `open`       | `boolean`                 | Open state   |
| `setOpen`    | `(open: boolean) => void` | Control open |
| `reposition` | `(options?) => void`      | Reposition   |

---

## 27. Progress (Linear)

**Ark-UI name:** `Progress`
**Category:** Data Display

### 27.1 Anatomy (Parts)

- `Progress.Root`, `.Label`, `.ValueText`
- `Progress.Track`, `.Range`
- `Progress.RootProvider`

### 27.2 Root Props

| Prop            | Type                                    | Default                | Description      |
| --------------- | --------------------------------------- | ---------------------- | ---------------- |
| `defaultValue`  | `number`                                | `50`                   | Initial value    |
| `formatOptions` | `NumberFormatOptions`                   | `{ style: 'percent' }` | Formatting       |
| `id`            | `string`                                | —                      | Unique ID        |
| `ids`           | `Partial<{root, track, label, circle}>` | —                      | Custom IDs       |
| `locale`        | `string`                                | `'en-US'`              | Locale           |
| `max`           | `number`                                | `100`                  | Maximum          |
| `min`           | `number`                                | `0`                    | Minimum          |
| `onValueChange` | `(details: ValueChangeDetails) => void` | —                      | Value change     |
| `orientation`   | `'horizontal' \| 'vertical'`            | `'horizontal'`         | Layout           |
| `translations`  | `IntlTranslations`                      | —                      | i18n             |
| `value`         | `number`                                | —                      | Controlled value |

### 27.3 Data Attributes

| Attribute            | Values           |
| -------------------- | ---------------- |
| `[data-max]`         | Maximum value    |
| `[data-value]`       | Current value    |
| `[data-state]`       | State indicator  |
| `[data-orientation]` | Layout direction |

### 27.4 CSS Variables

| Variable    | Description      |
| ----------- | ---------------- |
| `--percent` | Percentage value |

---

## 28. Progress (Circular)

**Ark-UI name:** `Progress` (circular variant)
**Category:** Data Display

### 28.1 Anatomy (Parts)

- `Progress.Root`, `.Label`, `.ValueText`
- `Progress.Circle`, `.CircleTrack`, `.CircleRange`
- `Progress.RootProvider`

### 28.2 Root Props

Same as Linear Progress.

### 28.3 CSS Variables

| Variable          | Description      |
| ----------------- | ---------------- |
| `--percent`       | Percentage       |
| `--radius`        | Border radius    |
| `--circumference` | Circle perimeter |
| `--offset`        | Stroke offset    |

---

## 29. QR Code

**Ark-UI name:** `QrCode`
**Category:** Specialized

### 29.1 Anatomy (Parts)

- `QrCode.Root`, `.Frame` (`<svg>`), `.Pattern` (`<svgpath>`)
- `QrCode.Overlay` (`<div>`), `.DownloadTrigger`
- `QrCode.RootProvider`

### 29.2 Root Props

| Prop            | Type                                    | Default | Description           |
| --------------- | --------------------------------------- | ------- | --------------------- |
| `defaultValue`  | `string`                                | —       | Initial encoded value |
| `encoding`      | `QrCodeGenerateOptions`                 | —       | Generation options    |
| `id`            | `string`                                | —       | Unique ID             |
| `ids`           | `Partial<{root, frame}>`                | —       | Custom IDs            |
| `onValueChange` | `(details: ValueChangeDetails) => void` | —       | Value change          |
| `pixelSize`     | `number`                                | —       | Pixel dimensions      |
| `value`         | `string`                                | —       | Controlled value      |

### 29.3 DownloadTrigger Props

| Prop       | Type          | Description                |
| ---------- | ------------- | -------------------------- |
| `fileName` | `string`      | Output filename (required) |
| `mimeType` | `DataUrlType` | Image format (required)    |
| `quality`  | `number`      | Image quality              |

### 29.4 CSS Variables

| Variable              | Description  |
| --------------------- | ------------ |
| `--qrcode-pixel-size` | Pixel size   |
| `--qrcode-width`      | Total width  |
| `--qrcode-height`     | Total height |

### 29.5 Context API

| Property     | Type                                  | Description   |
| ------------ | ------------------------------------- | ------------- |
| `value`      | `string`                              | Encoded value |
| `setValue`   | `(value: string) => void`             | Update value  |
| `getDataUrl` | `(type, quality?) => Promise<string>` | Export as URL |

---

## 30. Radio Group

**Ark-UI name:** `RadioGroup`
**Category:** Input

### 30.1 Anatomy (Parts)

- `RadioGroup.Root`, `.Label`, `.Indicator`
- `RadioGroup.Item`, `.ItemControl`, `.ItemText`, `.ItemHiddenInput`
- `RadioGroup.RootProvider`

### 30.2 Root Props

| Prop            | Type                                    | Default | Description      |
| --------------- | --------------------------------------- | ------- | ---------------- |
| `defaultValue`  | `string`                                | —       | Initial value    |
| `disabled`      | `boolean`                               | —       | Disabled         |
| `form`          | `string`                                | —       | Form ID          |
| `id`            | `string`                                | —       | Unique ID        |
| `ids`           | `Partial<{...}>`                        | —       | Custom IDs       |
| `invalid`       | `boolean`                               | —       | Invalid          |
| `name`          | `string`                                | —       | Form name        |
| `onValueChange` | `(details: ValueChangeDetails) => void` | —       | Change callback  |
| `orientation`   | `'horizontal' \| 'vertical'`            | —       | Layout           |
| `readOnly`      | `boolean`                               | —       | Read-only        |
| `required`      | `boolean`                               | —       | Required         |
| `value`         | `string`                                | —       | Controlled value |

### 30.3 Item Props

| Prop       | Type      | Description                |
| ---------- | --------- | -------------------------- |
| `value`    | `string`  | Selection value (required) |
| `disabled` | `boolean` | Individual disabled        |
| `invalid`  | `boolean` | Individual invalid         |

### 30.4 Indicator CSS Variables

| Variable                | Description |
| ----------------------- | ----------- |
| `--transition-property` | Animation   |
| `--left`, `--top`       | Position    |
| `--width`, `--height`   | Dimensions  |

### 30.5 Context API

| Property       | Type                      | Description       |
| -------------- | ------------------------- | ----------------- |
| `value`        | `string`                  | Current selection |
| `setValue`     | `(value: string) => void` | Update            |
| `clearValue`   | `VoidFunction`            | Clear             |
| `focus`        | `VoidFunction`            | Focus group       |
| `getItemState` | `(props) => ItemState`    | Get state         |

---

## 31. Rating Group

**Ark-UI name:** `RatingGroup`
**Category:** Data Display

### 31.1 Anatomy (Parts)

- `RatingGroup.Root`, `.Label`, `.Control`
- `RatingGroup.Item`, `.ItemContext`, `.HiddenInput`
- `RatingGroup.RootProvider`

### 31.2 Root Props

| Prop           | Type                                                 | Default | Description          |
| -------------- | ---------------------------------------------------- | ------- | -------------------- |
| `allowHalf`    | `boolean`                                            | —       | Half-star selections |
| `autoFocus`    | `boolean`                                            | —       | Auto-focus           |
| `count`        | `number`                                             | `5`     | Total items          |
| `defaultValue` | `number`                                             | —       | Initial value        |
| `disabled`     | `boolean`                                            | —       | Disabled             |
| `form`         | `string`                                             | —       | Form ID              |
| `id`           | `string`                                             | —       | Unique ID            |
| `ids`          | `Partial<{root, label, hiddenInput, control, item}>` | —       | Custom IDs           |
| `name`         | `string`                                             | —       | Form name            |
| `readOnly`     | `boolean`                                            | —       | Read-only            |
| `required`     | `boolean`                                            | —       | Required             |
| `translations` | `IntlTranslations`                                   | —       | i18n                 |
| `value`        | `number`                                             | —       | Controlled value     |

### 31.3 Events

| Event           | Payload              | Description   |
| --------------- | -------------------- | ------------- |
| `onHoverChange` | `HoverChangeDetails` | Hover changed |
| `onValueChange` | `ValueChangeDetails` | Value changed |

### 31.4 Item Props

| Prop    | Type     | Description           |
| ------- | -------- | --------------------- |
| `index` | `number` | Item index (required) |

### 31.5 Data Attributes

| Part | Attribute            | Values          |
| ---- | -------------------- | --------------- |
| Item | `[data-checked]`     | Selected        |
| Item | `[data-highlighted]` | Hovered/focused |
| Item | `[data-half]`        | Half-star       |

### 31.6 Context API

| Property       | Type                      | Description   |
| -------------- | ------------------------- | ------------- |
| `setValue`     | `(value: number) => void` | Set value     |
| `clearValue`   | `VoidFunction`            | Reset         |
| `hovering`     | `boolean`                 | Hover state   |
| `value`        | `number`                  | Current value |
| `hoveredValue` | `number`                  | Hovered value |
| `count`        | `number`                  | Total items   |

---

## 32. Scroll Area

**Ark-UI name:** `ScrollArea`
**Category:** Layout

### 32.1 Anatomy (Parts)

- `ScrollArea.Root`, `.Viewport`, `.Content`
- `ScrollArea.Scrollbar`, `.Thumb`, `.Corner`
- `ScrollArea.RootProvider`

### 32.2 Root Props

| Prop      | Type                                                   | Description |
| --------- | ------------------------------------------------------ | ----------- |
| `asChild` | `boolean`                                              | Composition |
| `ids`     | `Partial<{root, viewport, content, scrollbar, thumb}>` | Custom IDs  |

### 32.3 Scrollbar Props

| Prop          | Type          | Description      |
| ------------- | ------------- | ---------------- |
| `orientation` | `Orientation` | Scroll direction |
| `asChild`     | `boolean`     | Composition      |

### 32.4 Data Attributes

| Part      | Attribute                                                                | Values                  |
| --------- | ------------------------------------------------------------------------ | ----------------------- |
| Root      | `[data-overflow-x]`, `[data-overflow-y]`                                 | Present when overflow   |
| Viewport  | `[data-at-top]`, `[data-at-bottom]`, `[data-at-left]`, `[data-at-right]` | Scroll position         |
| Scrollbar | `[data-orientation]`                                                     | Direction               |
| Scrollbar | `[data-scrolling]`                                                       | Active scrolling        |
| Scrollbar | `[data-dragging]`, `[data-hover]`                                        | Interaction state       |
| Corner    | `[data-state]`                                                           | `"hidden" \| "visible"` |

### 32.5 CSS Variables

| Variable                                 | Description          |
| ---------------------------------------- | -------------------- |
| `--scroll-area-overflow-x-start`, `-end` | Horizontal distances |
| `--scroll-area-overflow-y-start`, `-end` | Vertical distances   |
| `--corner-width`, `--corner-height`      | Corner dimensions    |
| `--thumb-width`, `--thumb-height`        | Thumb dimensions     |

### 32.6 Context API

| Property                                         | Type                | Description           |
| ------------------------------------------------ | ------------------- | --------------------- |
| `isAtTop`, `isAtBottom`, `isAtLeft`, `isAtRight` | `boolean`           | Position state        |
| `hasOverflowX`, `hasOverflowY`                   | `boolean`           | Overflow state        |
| `getScrollProgress`                              | `() => Point`       | Progress (0-1)        |
| `scrollToEdge`                                   | `(details) => void` | Navigate to edge      |
| `scrollTo`                                       | `(details) => void` | Scroll to coordinates |

---

## 33. Segment Group

**Ark-UI name:** `SegmentGroup`
**Category:** Selection

### 33.1 Anatomy (Parts)

- `SegmentGroup.Root`, `.Label`, `.Indicator`
- `SegmentGroup.Item`, `.ItemText`, `.ItemControl`, `.ItemHiddenInput`
- `SegmentGroup.RootProvider`

### 33.2 Root Props

| Prop            | Type                                    | Default | Description      |
| --------------- | --------------------------------------- | ------- | ---------------- |
| `defaultValue`  | `string`                                | —       | Initial value    |
| `disabled`      | `boolean`                               | —       | Disabled         |
| `form`          | `string`                                | —       | Form ID          |
| `id`            | `string`                                | —       | Unique ID        |
| `ids`           | `Partial<{...}>`                        | —       | Custom IDs       |
| `invalid`       | `boolean`                               | —       | Invalid          |
| `name`          | `string`                                | —       | Form name        |
| `onValueChange` | `(details: ValueChangeDetails) => void` | —       | Change callback  |
| `orientation`   | `'horizontal' \| 'vertical'`            | —       | Layout           |
| `readOnly`      | `boolean`                               | —       | Read-only        |
| `required`      | `boolean`                               | —       | Required         |
| `value`         | `string`                                | —       | Controlled value |

### 33.3 Item Props

| Prop       | Type      | Description                |
| ---------- | --------- | -------------------------- |
| `value`    | `string`  | Selection value (required) |
| `disabled` | `boolean` | Disabled                   |
| `invalid`  | `boolean` | Invalid                    |

### 33.4 Indicator CSS Variables

| Variable                | Description |
| ----------------------- | ----------- |
| `--transition-property` | Animation   |
| `--left`, `--top`       | Position    |
| `--width`, `--height`   | Dimensions  |

---

## 34. Select

**Ark-UI name:** `Select`
**Category:** Selection

### 34.1 Anatomy (Parts)

- `Select.Root`, `.Label`, `.Control`, `.Trigger`
- `Select.ValueText`, `.ClearTrigger`, `.Indicator`
- `Select.Positioner`, `.Content`
- `Select.ItemGroup`, `.ItemGroupLabel`
- `Select.Item`, `.ItemText`, `.ItemIndicator`
- `Select.HiddenSelect`, `.List`, `.RootProvider`

### 34.2 Root Props

| Prop                      | Type                 | Default | Description                |
| ------------------------- | -------------------- | ------- | -------------------------- |
| `collection`              | `ListCollection<T>`  | —       | Item collection (required) |
| `value`                   | `string[]`           | —       | Controlled selection       |
| `defaultValue`            | `string[]`           | —       | Initial selection          |
| `open`                    | `boolean`            | —       | Controlled open            |
| `defaultOpen`             | `boolean`            | —       | Initial open               |
| `highlightedValue`        | `string`             | —       | Controlled highlight       |
| `defaultHighlightedValue` | `string`             | —       | Initial highlight          |
| `multiple`                | `boolean`            | —       | Multiple selection         |
| `disabled`                | `boolean`            | —       | Disabled                   |
| `readOnly`                | `boolean`            | —       | Read-only                  |
| `required`                | `boolean`            | —       | Required                   |
| `invalid`                 | `boolean`            | —       | Invalid                    |
| `deselectable`            | `boolean`            | —       | Allow deselect             |
| `closeOnSelect`           | `boolean`            | `true`  | Close after select         |
| `composite`               | `boolean`            | `true`  | Composed widget            |
| `loopFocus`               | `boolean`            | `false` | Loop navigation            |
| `lazyMount`               | `boolean`            | `false` | Lazy mount                 |
| `unmountOnExit`           | `boolean`            | `false` | Unmount on close           |
| `name`                    | `string`             | —       | Form name                  |
| `form`                    | `string`             | —       | Form ID                    |
| `autoComplete`            | `string`             | —       | Browser autofill           |
| `positioning`             | `PositioningOptions` | —       | Position config            |
| `scrollToIndexFn`         | `(details) => void`  | —       | Scroll function            |

### 34.3 Events

| Event                  | Payload                     | Description         |
| ---------------------- | --------------------------- | ------------------- |
| `onValueChange`        | `ValueChangeDetails<T>`     | Selection changed   |
| `onOpenChange`         | `OpenChangeDetails`         | Open state change   |
| `onHighlightChange`    | `HighlightChangeDetails<T>` | Highlight change    |
| `onSelect`             | `SelectionDetails`          | Item selected       |
| `onFocusOutside`       | `FocusOutsideEvent`         | Focus outside       |
| `onInteractOutside`    | `InteractOutsideEvent`      | Interaction outside |
| `onPointerDownOutside` | `PointerDownOutsideEvent`   | Pointer outside     |
| `onExitComplete`       | —                           | Animation complete  |

### 34.4 Item Props

| Prop           | Type      | Description                  |
| -------------- | --------- | ---------------------------- |
| `item`         | `any`     | Item data                    |
| `persistFocus` | `boolean` | Keep highlight on hover exit |

### 34.5 ValueText Props

| Prop          | Type     | Description      |
| ------------- | -------- | ---------------- |
| `placeholder` | `string` | Placeholder text |

### 34.6 Data Attributes

| Part    | Attribute                      | Values                     |
| ------- | ------------------------------ | -------------------------- |
| Trigger | `[data-state]`                 | `"open" \| "closed"`       |
| Trigger | `[data-placeholder-shown]`     | Present when empty         |
| Control | `[data-state]`, `[data-focus]` | State indicators           |
| Item    | `[data-value]`, `[data-state]` | `"checked" \| "unchecked"` |
| Item    | `[data-highlighted]`           | Present when focused       |

### 34.7 Context API

| Property           | Type                       | Description       |
| ------------------ | -------------------------- | ----------------- |
| `value`            | `string[]`                 | Selected keys     |
| `selectedItems`    | `V[]`                      | Selected items    |
| `highlightedValue` | `string`                   | Current highlight |
| `open`             | `boolean`                  | Open state        |
| `focused`          | `boolean`                  | Focus state       |
| `empty`            | `boolean`                  | No selection      |
| `selectValue`      | `(value: string) => void`  | Select            |
| `selectAll`        | `VoidFunction`             | Select all        |
| `clearValue`       | `(value?: string) => void` | Clear             |
| `setOpen`          | `(open: boolean) => void`  | Control menu      |
| `collection`       | `ListCollection<V>`        | Collection        |

---

## 35. Signature Pad

**Ark-UI name:** `SignaturePad`
**Category:** Specialized

### 35.1 Anatomy (Parts)

- `SignaturePad.Root`, `.Label`, `.Control`
- `SignaturePad.Segment` (`<svg>`), `.ClearTrigger`, `.Guide`
- `SignaturePad.HiddenInput`, `.RootProvider`

### 35.2 Root Props

| Prop           | Type                                           | Default                               | Description      |
| -------------- | ---------------------------------------------- | ------------------------------------- | ---------------- |
| `defaultPaths` | `string[]`                                     | —                                     | Initial paths    |
| `disabled`     | `boolean`                                      | —                                     | Disabled         |
| `drawing`      | `DrawingOptions`                               | `{ size: 2, simulatePressure: true }` | Pen config       |
| `ids`          | `Partial<{root, control, hiddenInput, label}>` | —                                     | Custom IDs       |
| `name`         | `string`                                       | —                                     | Form name        |
| `paths`        | `string[]`                                     | —                                     | Controlled paths |
| `readOnly`     | `boolean`                                      | —                                     | Read-only        |
| `required`     | `boolean`                                      | —                                     | Required         |
| `translations` | `IntlTranslations`                             | —                                     | i18n             |

### 35.3 Events

| Event       | Payload          | Description      |
| ----------- | ---------------- | ---------------- |
| `onDraw`    | `DrawDetails`    | During drawing   |
| `onDrawEnd` | `DrawEndDetails` | Drawing complete |

### 35.4 Context API

| Property      | Type                                  | Description     |
| ------------- | ------------------------------------- | --------------- |
| `empty`       | `boolean`                             | No strokes      |
| `drawing`     | `boolean`                             | Active drawing  |
| `currentPath` | `string`                              | Active stroke   |
| `paths`       | `string[]`                            | All paths       |
| `getDataUrl`  | `(type, quality?) => Promise<string>` | Export as image |
| `clear`       | `VoidFunction`                        | Reset all       |

---

## 36. Slider

**Ark-UI name:** `Slider`
**Category:** Input

Note: Ark-UI uses a single Slider component for both single-value and range (multi-thumb) sliders. Values are always `number[]`.

### 36.1 Anatomy (Parts)

- `Slider.Root`, `.Label`, `.ValueText`
- `Slider.Control`, `.Track`, `.Range`, `.Thumb`
- `Slider.HiddenInput`, `.MarkerGroup`, `.Marker`
- `Slider.DraggingIndicator`, `.RootProvider`

### 36.2 Root Props

| Prop                     | Type                                | Default        | Description            |
| ------------------------ | ----------------------------------- | -------------- | ---------------------- |
| `aria-label`             | `string[]`                          | —              | Labels per thumb       |
| `aria-labelledby`        | `string[]`                          | —              | Label IDs per thumb    |
| `defaultValue`           | `number[]`                          | —              | Initial values         |
| `disabled`               | `boolean`                           | —              | Disabled               |
| `form`                   | `string`                            | —              | Form ID                |
| `getAriaValueText`       | `(details) => string`               | —              | Value formatter        |
| `id`                     | `string`                            | —              | Unique ID              |
| `ids`                    | `Partial<{...}>`                    | —              | Custom IDs             |
| `invalid`                | `boolean`                           | —              | Invalid                |
| `max`                    | `number`                            | `100`          | Maximum                |
| `min`                    | `number`                            | `0`            | Minimum                |
| `minStepsBetweenThumbs`  | `number`                            | `0`            | Min gap between thumbs |
| `name`                   | `string`                            | —              | Form name              |
| `orientation`            | `'horizontal' \| 'vertical'`        | `'horizontal'` | Layout                 |
| `origin`                 | `'center' \| 'start' \| 'end'`      | `'start'`      | Track fill origin      |
| `readOnly`               | `boolean`                           | —              | Read-only              |
| `step`                   | `number`                            | `1`            | Step amount            |
| `thumbAlignment`         | `'center' \| 'contain'`             | `'contain'`    | Thumb positioning      |
| `thumbCollisionBehavior` | `'none' \| 'push' \| 'swap'`        | `'none'`       | Multi-thumb collision  |
| `thumbSize`              | `{ width: number; height: number }` | —              | Thumb dimensions       |
| `value`                  | `number[]`                          | —              | Controlled values      |

### 36.3 Events

| Event              | Payload              | Description     |
| ------------------ | -------------------- | --------------- |
| `onFocusChange`    | `FocusChangeDetails` | Focus change    |
| `onValueChange`    | `ValueChangeDetails` | Value change    |
| `onValueChangeEnd` | `ValueChangeDetails` | Change complete |

### 36.4 Data Attributes

| Attribute            | Values                |
| -------------------- | --------------------- |
| `[data-disabled]`    | Present when disabled |
| `[data-orientation]` | Direction             |
| `[data-dragging]`    | During drag           |
| `[data-invalid]`     | When invalid          |
| `[data-focus]`       | When focused          |

### 36.5 CSS Variables

| Variable                                        | Description       |
| ----------------------------------------------- | ----------------- |
| `--slider-thumb-width`, `--slider-thumb-height` | Thumb dimensions  |
| `--slider-thumb-transform`                      | Thumb positioning |
| `--slider-range-start`, `--slider-range-end`    | Range positions   |

### 36.6 Context API

| Property          | Type                                     | Description       |
| ----------------- | ---------------------------------------- | ----------------- |
| `value`           | `number[]`                               | Values            |
| `dragging`        | `boolean`                                | Drag state        |
| `focused`         | `boolean`                                | Focus state       |
| `setValue`        | `(value: number[]) => void`              | Set all values    |
| `getThumbValue`   | `(index: number) => number`              | Get thumb value   |
| `setThumbValue`   | `(index: number, value: number) => void` | Set thumb         |
| `getValuePercent` | `(value: number) => number`              | Value to percent  |
| `getPercentValue` | `(percent: number) => number`            | Percent to value  |
| `increment`       | `(index: number) => void`                | Increment         |
| `decrement`       | `(index: number) => void`                | Decrement         |
| `focus`           | `VoidFunction`                           | Focus first thumb |

---

## 37. Splitter

**Ark-UI name:** `Splitter`
**Category:** Layout

### 37.1 Anatomy (Parts)

- `Splitter.Root`, `.Panel`
- `Splitter.ResizeTrigger`, `.ResizeTriggerIndicator`
- `Splitter.RootProvider`

### 37.2 Root Props

| Prop               | Type                         | Default        | Description               |
| ------------------ | ---------------------------- | -------------- | ------------------------- |
| `panels`           | `PanelData[]`                | —              | Panel size constraints    |
| `defaultSize`      | `number[]`                   | —              | Initial sizes             |
| `size`             | `number[]`                   | —              | Controlled sizes          |
| `orientation`      | `'horizontal' \| 'vertical'` | `'horizontal'` | Layout                    |
| `keyboardResizeBy` | `number`                     | —              | Keyboard resize step (px) |
| `id`               | `string`                     | —              | Unique ID                 |
| `ids`              | `Partial<{...}>`             | —              | Custom IDs                |
| `nonce`            | `string`                     | —              | Stylesheet nonce          |

### 37.3 Events

| Event           | Payload                 | Description     |
| --------------- | ----------------------- | --------------- |
| `onResizeStart` | —                       | Resize started  |
| `onResize`      | `ResizeDetails`         | During resize   |
| `onResizeEnd`   | `ResizeEndDetails`      | Resize ended    |
| `onCollapse`    | `ExpandCollapseDetails` | Panel collapsed |
| `onExpand`      | `ExpandCollapseDetails` | Panel expanded  |

### 37.4 Panel Props

| Prop | Type     | Description      |
| ---- | -------- | ---------------- |
| `id` | `string` | Panel identifier |

### 37.5 ResizeTrigger Props

| Prop       | Type      | Description        |
| ---------- | --------- | ------------------ |
| `id`       | `string`  | Trigger identifier |
| `disabled` | `boolean` | Disabled           |

### 37.6 Data Attributes

| Part          | Attribute                         | Values         |
| ------------- | --------------------------------- | -------------- |
| Root          | `[data-orientation]`              | Direction      |
| Root          | `[data-dragging]`                 | During drag    |
| Panel         | `[data-id]`, `[data-index]`       | Identification |
| ResizeTrigger | `[data-disabled]`, `[data-focus]` | State          |

### 37.7 Context API

| Property           | Type                                     | Description   |
| ------------------ | ---------------------------------------- | ------------- |
| `dragging`         | `boolean`                                | Resize state  |
| `orientation`      | `string`                                 | Layout        |
| `getSizes`         | `() => number[]`                         | Current sizes |
| `setSizes`         | `(size: number[]) => void`               | Set sizes     |
| `getPanelSize`     | `(id: string) => number`                 | Panel size    |
| `isPanelCollapsed` | `(id: string) => boolean`                | Collapsed?    |
| `isPanelExpanded`  | `(id: string) => boolean`                | Expanded?     |
| `collapsePanel`    | `(id: string) => void`                   | Collapse      |
| `expandPanel`      | `(id: string, minSize?: number) => void` | Expand        |
| `resizePanel`      | `(id: string, size: number) => void`     | Resize        |
| `resetSizes`       | `VoidFunction`                           | Reset         |

---

## 38. Steps

**Ark-UI name:** `Steps`
**Category:** Navigation

### 38.1 Anatomy (Parts)

- `Steps.Root`, `.List`, `.Item`
- `Steps.Trigger`, `.Indicator`, `.Separator`
- `Steps.Content`, `.CompletedContent`
- `Steps.PrevTrigger`, `.NextTrigger`
- `Steps.RootProvider`

### 38.2 Root Props

| Prop              | Type                         | Default        | Description        |
| ----------------- | ---------------------------- | -------------- | ------------------ |
| `count`           | `number`                     | —              | Total steps        |
| `defaultStep`     | `number`                     | —              | Initial step       |
| `step`            | `number`                     | —              | Controlled step    |
| `linear`          | `boolean`                    | `false`        | Require sequential |
| `orientation`     | `'horizontal' \| 'vertical'` | `'horizontal'` | Layout             |
| `isStepValid`     | `(index: number) => boolean` | `() => true`   | Validate step      |
| `isStepSkippable` | `(index: number) => boolean` | `() => false`  | Skip allowed?      |

### 38.3 Events

| Event            | Payload              | Description  |
| ---------------- | -------------------- | ------------ |
| `onStepChange`   | `StepChangeDetails`  | Step changed |
| `onStepComplete` | —                    | Completed    |
| `onStepInvalid`  | `StepInvalidDetails` | Blocked      |

### 38.4 Item Props

| Prop    | Type     | Description   |
| ------- | -------- | ------------- |
| `index` | `number` | Step position |

### 38.5 Content Props

| Prop    | Type     | Description     |
| ------- | -------- | --------------- |
| `index` | `number` | Associated step |

### 38.6 Data Attributes

| Part      | Attribute                           | Values               |
| --------- | ----------------------------------- | -------------------- |
| Root      | `[data-orientation]`                | Direction            |
| Content   | `[data-state]`                      | `"open" \| "closed"` |
| Indicator | `[data-complete]`, `[data-current]` | State                |
| Separator | `[data-complete]`                   | Done                 |

### 38.7 CSS Variables

| Variable    | Description |
| ----------- | ----------- |
| `--percent` | Completion  |

### 38.8 Context API

| Property       | Type                     | Description  |
| -------------- | ------------------------ | ------------ |
| `value`        | `number`                 | Current step |
| `percent`      | `number`                 | Completion % |
| `count`        | `number`                 | Total        |
| `hasNextStep`  | `boolean`                | Can advance  |
| `hasPrevStep`  | `boolean`                | Can go back  |
| `isCompleted`  | `boolean`                | All done     |
| `setStep`      | `(step: number) => void` | Navigate     |
| `goToNextStep` | `VoidFunction`           | Forward      |
| `goToPrevStep` | `VoidFunction`           | Backward     |
| `resetStep`    | `VoidFunction`           | Reset        |

---

## 39. Switch

**Ark-UI name:** `Switch`
**Category:** Input

### 39.1 Anatomy (Parts)

- `Switch.Root` (`<label>`), `.Control` (`<span>`)
- `Switch.Thumb` (`<span>`), `.Label` (`<span>`)
- `Switch.HiddenInput` (`<input>`), `.RootProvider`

### 39.2 Root Props

| Prop              | Type                                                  | Default | Description         |
| ----------------- | ----------------------------------------------------- | ------- | ------------------- |
| `checked`         | `boolean`                                             | —       | Controlled state    |
| `disabled`        | `boolean`                                             | —       | Disabled            |
| `ids`             | `Partial<{root, hiddenInput, control, label, thumb}>` | —       | Custom IDs          |
| `invalid`         | `boolean`                                             | —       | Invalid             |
| `label`           | `string`                                              | —       | Accessibility label |
| `name`            | `string`                                              | —       | Form name           |
| `onCheckedChange` | `(details) => void`                                   | —       | Change callback     |
| `readOnly`        | `boolean`                                             | —       | Read-only           |
| `required`        | `boolean`                                             | —       | Required            |
| `value`           | `string \| number`                                    | `'on'`  | Form value          |

### 39.3 Data Attributes

| Attribute              | Values                     |
| ---------------------- | -------------------------- |
| `[data-active]`        | When pressed               |
| `[data-focus]`         | When focused               |
| `[data-focus-visible]` | Keyboard focus             |
| `[data-readonly]`      | Read-only                  |
| `[data-hover]`         | On hover                   |
| `[data-disabled]`      | Disabled                   |
| `[data-state]`         | `"checked" \| "unchecked"` |
| `[data-invalid]`       | Invalid                    |
| `[data-required]`      | Required                   |

### 39.4 Context API

| Property        | Type                         | Description |
| --------------- | ---------------------------- | ----------- |
| `checked`       | `boolean`                    | State       |
| `disabled`      | `boolean`                    | Disabled    |
| `focused`       | `boolean`                    | Focused     |
| `setChecked`    | `(checked: boolean) => void` | Set state   |
| `toggleChecked` | `VoidFunction`               | Toggle      |

---

## 40. Tabs

**Ark-UI name:** `Tabs`
**Category:** Navigation

### 40.1 Anatomy (Parts)

- `Tabs.Root`, `.List`, `.Trigger`
- `Tabs.Content`, `.Indicator`, `.RootProvider`

### 40.2 Root Props

| Prop             | Type                                                 | Default        | Description               |
| ---------------- | ---------------------------------------------------- | -------------- | ------------------------- |
| `activationMode` | `'manual' \| 'automatic'`                            | `'automatic'`  | Focus vs click activation |
| `composite`      | `boolean`                                            | —              | Composite widget          |
| `defaultValue`   | `string`                                             | —              | Initial tab               |
| `deselectable`   | `boolean`                                            | —              | Allow deselect            |
| `id`             | `string`                                             | —              | Unique ID                 |
| `ids`            | `Partial<{root, trigger, list, content, indicator}>` | —              | Custom IDs                |
| `lazyMount`      | `boolean`                                            | `false`        | Lazy mount                |
| `loopFocus`      | `boolean`                                            | `true`         | Loop navigation           |
| `navigate`       | `(details: NavigateDetails) => void`                 | —              | Custom navigation         |
| `orientation`    | `'horizontal' \| 'vertical'`                         | `'horizontal'` | Layout                    |
| `translations`   | `IntlTranslations`                                   | —              | i18n                      |
| `unmountOnExit`  | `boolean`                                            | `false`        | Unmount on switch         |
| `value`          | `string`                                             | —              | Controlled tab            |

### 40.3 Events

| Event           | Payload              | Description   |
| --------------- | -------------------- | ------------- |
| `onFocusChange` | `FocusChangeDetails` | Focus changed |
| `onValueChange` | `ValueChangeDetails` | Tab changed   |

### 40.4 Trigger Props

| Prop       | Type      | Description       |
| ---------- | --------- | ----------------- |
| `value`    | `string`  | Tab ID (required) |
| `disabled` | `boolean` | Disabled          |

### 40.5 Content Props

| Prop    | Type     | Description                  |
| ------- | -------- | ---------------------------- |
| `value` | `string` | Associated tab ID (required) |

### 40.6 Context API

| Property          | Type                           | Description   |
| ----------------- | ------------------------------ | ------------- |
| `value`           | `string`                       | Selected tab  |
| `focusedValue`    | `string`                       | Focused tab   |
| `setValue`        | `(value: string) => void`      | Select tab    |
| `clearValue`      | `VoidFunction`                 | Clear         |
| `focus`           | `VoidFunction`                 | Focus trigger |
| `selectNext`      | `(fromValue?: string) => void` | Next tab      |
| `selectPrev`      | `(fromValue?: string) => void` | Previous tab  |
| `getTriggerState` | `(props) => TriggerState`      | Get state     |

---

## 41. Tags Input

**Ark-UI name:** `TagsInput`
**Category:** Selection

### 41.1 Anatomy (Parts)

- `TagsInput.Root`, `.Label`, `.Control`
- `TagsInput.Item`, `.ItemPreview`, `.ItemText`, `.ItemDeleteTrigger`, `.ItemInput`
- `TagsInput.Input`, `.ClearTrigger`, `.HiddenInput`
- `TagsInput.RootProvider`

### 41.2 Root Props

| Prop                | Type                   | Default    | Description         |
| ------------------- | ---------------------- | ---------- | ------------------- |
| `addOnPaste`        | `boolean`              | `false`    | Paste creates tags  |
| `allowOverflow`     | `boolean`              | —          | Allow exceeding max |
| `autoFocus`         | `boolean`              | —          | Auto-focus          |
| `blurBehavior`      | `'clear' \| 'add'`     | —          | Blur action         |
| `defaultInputValue` | `string`               | —          | Initial input       |
| `defaultValue`      | `string[]`             | —          | Initial tags        |
| `delimiter`         | `string \| RegExp`     | `','`      | Tag delimiter       |
| `disabled`          | `boolean`              | —          | Disabled            |
| `editable`          | `boolean`              | `true`     | Allow editing       |
| `form`              | `string`               | —          | Form ID             |
| `id`                | `string`               | —          | Unique ID           |
| `ids`               | `Partial<{...}>`       | —          | Custom IDs          |
| `inputValue`        | `string`               | —          | Controlled input    |
| `invalid`           | `boolean`              | —          | Invalid             |
| `max`               | `number`               | `Infinity` | Max tags            |
| `maxLength`         | `number`               | —          | Max chars per tag   |
| `name`              | `string`               | —          | Form name           |
| `placeholder`       | `string`               | —          | Placeholder         |
| `readOnly`          | `boolean`              | —          | Read-only           |
| `required`          | `boolean`              | —          | Required            |
| `validate`          | `(details) => boolean` | —          | Custom validation   |
| `value`             | `string[]`             | —          | Controlled tags     |

### 41.3 Events

| Event                  | Payload                   | Description           |
| ---------------------- | ------------------------- | --------------------- |
| `onValueChange`        | `ValueChangeDetails`      | Tags changed          |
| `onInputValueChange`   | `InputValueChangeDetails` | Input changed         |
| `onHighlightChange`    | `HighlightChangeDetails`  | Tag highlight changed |
| `onValueInvalid`       | `ValidityChangeDetails`   | Validation failed     |
| `onFocusOutside`       | `FocusOutsideEvent`       | Focus outside         |
| `onInteractOutside`    | `InteractOutsideEvent`    | Interaction outside   |
| `onPointerDownOutside` | `PointerDownOutsideEvent` | Pointer outside       |

### 41.4 Item Props

| Prop       | Type               | Description  |
| ---------- | ------------------ | ------------ |
| `index`    | `string \| number` | Tag position |
| `value`    | `string`           | Tag content  |
| `disabled` | `boolean`          | Disabled     |

### 41.5 Data Attributes

| Part | Attribute            | Values    |
| ---- | -------------------- | --------- |
| Root | `[data-empty]`       | No tags   |
| Root | `[data-focus]`       | Focused   |
| Item | `[data-value]`       | Tag value |
| Item | `[data-highlighted]` | Selected  |

### 41.6 Context API

| Property          | Type                                     | Description     |
| ----------------- | ---------------------------------------- | --------------- |
| `empty`           | `boolean`                                | No tags         |
| `inputValue`      | `string`                                 | Input text      |
| `value`           | `string[]`                               | Tag array       |
| `valueAsString`   | `string`                                 | Comma-separated |
| `count`           | `number`                                 | Tag count       |
| `atMax`           | `boolean`                                | At limit        |
| `setValue`        | `(value: string[]) => void`              | Replace all     |
| `addValue`        | `(value: string) => void`                | Add tag         |
| `clearValue`      | `(id?) => void`                          | Remove          |
| `setValueAtIndex` | `(index: number, value: string) => void` | Update          |

---

## 42. Timer

**Ark-UI name:** `Timer`
**Category:** Specialized

### 42.1 Anatomy (Parts)

- `Timer.Root`, `.Area`, `.Item`, `.Separator`
- `Timer.Control`, `.ActionTrigger`, `.RootProvider`

### 42.2 Root Props

| Prop        | Type                    | Default | Description          |
| ----------- | ----------------------- | ------- | -------------------- |
| `autoStart` | `boolean`               | —       | Auto-start           |
| `countdown` | `boolean`               | `false` | Countdown mode       |
| `ids`       | `Partial<{root, area}>` | —       | Custom IDs           |
| `interval`  | `number`                | `1000`  | Update interval (ms) |
| `startMs`   | `number`                | —       | Start time (ms)      |
| `targetMs`  | `number`                | —       | Target time (ms)     |

### 42.3 Events

| Event        | Payload       | Description     |
| ------------ | ------------- | --------------- |
| `onComplete` | —             | Timer completed |
| `onTick`     | `TickDetails` | Each tick       |

### 42.4 ActionTrigger Props

| Prop     | Type          | Description                                  |
| -------- | ------------- | -------------------------------------------- |
| `action` | `TimerAction` | Action: start, pause, resume, reset, restart |

### 42.5 Item Props

| Prop   | Type                 | Description                               |
| ------ | -------------------- | ----------------------------------------- |
| `type` | `keyof Time<number>` | Time unit (hours, minutes, seconds, etc.) |

### 42.6 Context API

| Property          | Type           | Description  |
| ----------------- | -------------- | ------------ |
| `running`         | `boolean`      | Active       |
| `paused`          | `boolean`      | Paused       |
| `time`            | `Time<number>` | Raw values   |
| `formattedTime`   | `Time<string>` | Formatted    |
| `start`           | `VoidFunction` | Start        |
| `pause`           | `VoidFunction` | Pause        |
| `resume`          | `VoidFunction` | Resume       |
| `reset`           | `VoidFunction` | Reset        |
| `restart`         | `VoidFunction` | Restart      |
| `progressPercent` | `number`       | Completion % |

---

## 43. Toast

**Ark-UI name:** `Toast` + `Toaster`
**Category:** Overlay

### 43.1 Anatomy (Parts)

- `Toaster` — container managing placement
- `Toast.Root`, `.Title`, `.Description`
- `Toast.ActionTrigger`, `.CloseTrigger`

### 43.2 Toaster Props

| Prop          | Type                  | Default | Description             |
| ------------- | --------------------- | ------- | ----------------------- |
| `toaster`     | `CreateToasterReturn` | —       | Toast engine (required) |
| `dir`         | `'ltr' \| 'rtl'`      | `'ltr'` | Direction               |
| `getRootNode` | `() => Node`          | —       | Custom root             |

### 43.3 createToaster Options

| Option            | Type               | Default    | Description       |
| ----------------- | ------------------ | ---------- | ----------------- |
| `duration`        | `number`           | —          | Auto-dismiss time |
| `gap`             | `number`           | `16`       | Toast gap (px)    |
| `hotkey`          | `string[]`         | —          | Focus trigger     |
| `max`             | `number`           | `24`       | Max toasts        |
| `offsets`         | `string \| Record` | `'1rem'`   | Edge offsets      |
| `overlap`         | `boolean`          | —          | Stack overlap     |
| `pauseOnPageIdle` | `boolean`          | `false`    | Pause on idle     |
| `placement`       | `Placement`        | `'bottom'` | Position          |
| `removeDelay`     | `number`           | `200`      | Remove delay (ms) |

### 43.4 Toast Methods

| Method                              | Description     |
| ----------------------------------- | --------------- |
| `toaster.create(options)`           | Generic toast   |
| `toaster.success(options)`          | Success toast   |
| `toaster.error(options)`            | Error toast     |
| `toaster.warning(options)`          | Warning toast   |
| `toaster.info(options)`             | Info toast      |
| `toaster.promise(promise, options)` | Async toast     |
| `toaster.update(id, options)`       | Update existing |

### 43.5 Data Attributes (Root)

| Attribute          | Values               |
| ------------------ | -------------------- |
| `[data-state]`     | State indicator      |
| `[data-type]`      | Toast type           |
| `[data-placement]` | Position             |
| `[data-mounted]`   | Present when mounted |
| `[data-paused]`    | Present when paused  |

---

## 44. Toggle

**Ark-UI name:** `Toggle`
**Category:** Utility

### 44.1 Anatomy (Parts)

- `Toggle.Root` (`<button>`), `.Indicator` (`<div>`)

### 44.2 Root Props

| Prop              | Type                         | Default | Description      |
| ----------------- | ---------------------------- | ------- | ---------------- |
| `defaultPressed`  | `boolean`                    | —       | Initial state    |
| `onPressedChange` | `(pressed: boolean) => void` | —       | Change callback  |
| `pressed`         | `boolean`                    | —       | Controlled state |

### 44.3 Indicator Props

| Prop       | Type        | Description              |
| ---------- | ----------- | ------------------------ |
| `fallback` | `ReactNode` | Content when not pressed |

### 44.4 Data Attributes

| Attribute         | Values                |
| ----------------- | --------------------- |
| `[data-state]`    | `"on" \| "off"`       |
| `[data-pressed]`  | Present when active   |
| `[data-disabled]` | Present when disabled |

### 44.5 Context API

| Property     | Type                         | Description |
| ------------ | ---------------------------- | ----------- |
| `pressed`    | `boolean`                    | State       |
| `disabled`   | `boolean`                    | Disabled    |
| `setPressed` | `(pressed: boolean) => void` | Set state   |

---

## 45. Toggle Group

**Ark-UI name:** `ToggleGroup`
**Category:** Utility

### 45.1 Anatomy (Parts)

- `ToggleGroup.Root` (`<div>`), `.Item` (`<button>`), `.RootProvider`

### 45.2 Root Props

| Prop            | Type                                    | Default        | Description       |
| --------------- | --------------------------------------- | -------------- | ----------------- |
| `defaultValue`  | `string[]`                              | —              | Initial selection |
| `deselectable`  | `boolean`                               | `true`         | Allow empty       |
| `disabled`      | `boolean`                               | —              | Disabled          |
| `id`            | `string`                                | —              | Unique ID         |
| `ids`           | `Partial<{root, item}>`                 | —              | Custom IDs        |
| `loopFocus`     | `boolean`                               | `true`         | Loop navigation   |
| `multiple`      | `boolean`                               | —              | Multi-selection   |
| `onValueChange` | `(details: ValueChangeDetails) => void` | —              | Change callback   |
| `orientation`   | `Orientation`                           | `'horizontal'` | Layout            |
| `rovingFocus`   | `boolean`                               | `true`         | Roving tabindex   |
| `value`         | `string[]`                              | —              | Controlled value  |

### 45.3 Item Props

| Prop       | Type      | Description           |
| ---------- | --------- | --------------------- |
| `value`    | `string`  | Identifier (required) |
| `disabled` | `boolean` | Disabled              |

### 45.4 Data Attributes

| Part | Attribute                         | Values          |
| ---- | --------------------------------- | --------------- |
| Root | `[data-orientation]`              | Direction       |
| Root | `[data-focus]`                    | Focused         |
| Item | `[data-state]`                    | `"on" \| "off"` |
| Item | `[data-focus]`, `[data-disabled]` | State           |

### 45.5 Context API

| Property       | Type                        | Description |
| -------------- | --------------------------- | ----------- |
| `value`        | `string[]`                  | Selection   |
| `setValue`     | `(value: string[]) => void` | Update      |
| `getItemState` | `(props) => ItemState`      | Item state  |

---

## 46. Tooltip

**Ark-UI name:** `Tooltip`
**Category:** Overlay

### 46.1 Anatomy (Parts)

- `Tooltip.Root`, `.Trigger`, `.Positioner`
- `Tooltip.Content`, `.Arrow`, `.ArrowTip`

### 46.2 Root Props

| Prop                   | Type                                             | Default | Description            |
| ---------------------- | ------------------------------------------------ | ------- | ---------------------- |
| `aria-label`           | `string`                                         | —       | Custom label           |
| `closeDelay`           | `number`                                         | `150`   | Close delay (ms)       |
| `closeOnClick`         | `boolean`                                        | `true`  | Close on click         |
| `closeOnEscape`        | `boolean`                                        | `true`  | Close on Escape        |
| `closeOnPointerDown`   | `boolean`                                        | `true`  | Close on pointer       |
| `closeOnScroll`        | `boolean`                                        | `true`  | Close on scroll        |
| `defaultOpen`          | `boolean`                                        | —       | Initial open           |
| `disabled`             | `boolean`                                        | —       | Disabled               |
| `id`                   | `string`                                         | —       | Unique ID              |
| `ids`                  | `Partial<{trigger, content, arrow, positioner}>` | —       | Custom IDs             |
| `immediate`            | `boolean`                                        | —       | Sync immediately       |
| `interactive`          | `boolean`                                        | `false` | Content stays on hover |
| `lazyMount`            | `boolean`                                        | `false` | Lazy mount             |
| `open`                 | `boolean`                                        | —       | Controlled open        |
| `openDelay`            | `number`                                         | `400`   | Open delay (ms)        |
| `positioning`          | `PositioningOptions`                             | —       | Position config        |
| `skipAnimationOnMount` | `boolean`                                        | `false` | Skip initial animation |
| `unmountOnExit`        | `boolean`                                        | `false` | Unmount on exit        |

### 46.3 Events

| Event            | Payload             | Description        |
| ---------------- | ------------------- | ------------------ |
| `onOpenChange`   | `OpenChangeDetails` | Open state change  |
| `onExitComplete` | —                   | Animation complete |

### 46.4 Data Attributes

| Part    | Attribute          | Values               |
| ------- | ------------------ | -------------------- |
| Content | `[data-state]`     | `"open" \| "closed"` |
| Content | `[data-placement]` | Position             |
| Content | `[data-instant]`   | Immediate open       |

### 46.5 Context API

| Property     | Type                      | Description |
| ------------ | ------------------------- | ----------- |
| `open`       | `boolean`                 | Open state  |
| `setOpen`    | `(open: boolean) => void` | Control     |
| `reposition` | `(options?) => void`      | Reposition  |

---

## 47. Tour

**Ark-UI name:** `Tour`
**Category:** Overlay

### 47.1 Anatomy (Parts)

- `Tour.Root`, `.Backdrop`, `.Spotlight`
- `Tour.Positioner`, `.Content`, `.Arrow`, `.ArrowTip`
- `Tour.Title`, `.Description`, `.ProgressText`
- `Tour.CloseTrigger`, `.Actions`, `.ActionTrigger`
- `Tour.Control`

### 47.2 Root Props

| Prop                   | Type            | Default | Description           |
| ---------------------- | --------------- | ------- | --------------------- |
| `tour`                 | `UseTourReturn` | —       | Tour state (required) |
| `present`              | `boolean`       | —       | Controlled visibility |
| `immediate`            | `boolean`       | —       | Sync immediately      |
| `lazyMount`            | `boolean`       | `false` | Lazy mount            |
| `unmountOnExit`        | `boolean`       | `false` | Unmount on close      |
| `skipAnimationOnMount` | `boolean`       | `false` | Skip animation        |
| `onExitComplete`       | `VoidFunction`  | —       | Animation complete    |

### 47.3 ActionTrigger Props

| Prop     | Type         | Description              |
| -------- | ------------ | ------------------------ |
| `action` | `StepAction` | Action config (required) |

### 47.4 Data Attributes

| Part     | Attribute          | Values               |
| -------- | ------------------ | -------------------- |
| Content  | `[data-state]`     | `"open" \| "closed"` |
| Content  | `[data-placement]` | Position             |
| Content  | `[data-step]`      | Current step ID      |
| Backdrop | `[data-type]`      | Step type            |

### 47.5 Context API

| Property                | Type           | Description   |
| ----------------------- | -------------- | ------------- |
| `open`                  | `boolean`      | Visibility    |
| `totalSteps`            | `number`       | Total steps   |
| `stepIndex`             | `number`       | Current index |
| `step`                  | `StepDetails`  | Current step  |
| `hasNextStep`           | `boolean`      | Can advance   |
| `hasPrevStep`           | `boolean`      | Can go back   |
| `firstStep`, `lastStep` | `boolean`      | Position      |
| `start`                 | `VoidFunction` | Start tour    |
| `next`                  | `VoidFunction` | Next step     |
| `prev`                  | `VoidFunction` | Previous step |
| `getProgressText`       | `() => string` | Progress text |
| `getProgressPercent`    | `() => number` | Progress %    |

---

## 48. Tree View

**Ark-UI name:** `TreeView`
**Category:** Navigation

### 48.1 Anatomy (Parts)

- `TreeView.Root`, `.Label`, `.Tree`
- `TreeView.NodeProvider`
- `TreeView.Branch`, `.BranchControl`, `.BranchIndicator`, `.BranchText`, `.BranchTrigger`, `.BranchContent`, `.BranchIndentGuide`
- `TreeView.Item`, `.ItemText`, `.ItemIndicator`
- `TreeView.NodeCheckbox`, `.NodeCheckboxIndicator`, `.NodeRenameInput`
- `TreeView.RootProvider`

### 48.2 Root Props

| Prop                   | Type                           | Default    | Description               |
| ---------------------- | ------------------------------ | ---------- | ------------------------- |
| `collection`           | `TreeCollection<T>`            | —          | Tree data (required)      |
| `canRename`            | `(node, indexPath) => boolean` | —          | Rename eligibility        |
| `checkedValue`         | `string[]`                     | —          | Controlled checkbox state |
| `defaultCheckedValue`  | `string[]`                     | —          | Initial checked           |
| `defaultExpandedValue` | `string[]`                     | —          | Initial expanded          |
| `defaultFocusedValue`  | `string`                       | —          | Initial focused           |
| `defaultSelectedValue` | `string[]`                     | —          | Initial selected          |
| `expandedValue`        | `string[]`                     | —          | Controlled expanded       |
| `expandOnClick`        | `boolean`                      | `true`     | Expand on click           |
| `focusedValue`         | `string`                       | —          | Controlled focus          |
| `lazyMount`            | `boolean`                      | `false`    | Lazy mount                |
| `unmountOnExit`        | `boolean`                      | `false`    | Unmount collapsed         |
| `selectionMode`        | `'single' \| 'multiple'`       | `'single'` | Selection                 |
| `typeahead`            | `boolean`                      | `true`     | Typeahead search          |
| `loadChildren`         | `(details) => Promise<T[]>`    | —          | Async loading             |
| `scrollToIndexFn`      | `(details) => void`            | —          | Virtualization            |

### 48.3 Events

| Event                    | Payload                          | Description      |
| ------------------------ | -------------------------------- | ---------------- |
| `onExpandedChange`       | `ExpandedChangeDetails<T>`       | Expand/collapse  |
| `onSelectionChange`      | `SelectionChangeDetails<T>`      | Selection change |
| `onCheckedChange`        | `CheckedChangeDetails`           | Checkbox change  |
| `onFocusChange`          | `FocusChangeDetails<T>`          | Focus change     |
| `onRenameStart`          | `RenameStartDetails<T>`          | Rename started   |
| `onRenameComplete`       | `RenameCompleteDetails`          | Rename done      |
| `onBeforeRename`         | `RenameCompleteDetails`          | Pre-rename       |
| `onLoadChildrenComplete` | `LoadChildrenCompleteDetails<T>` | Async done       |
| `onLoadChildrenError`    | `LoadChildrenErrorDetails<T>`    | Async error      |

### 48.4 Data Attributes

| Part         | Attribute                                     | Values                                        |
| ------------ | --------------------------------------------- | --------------------------------------------- |
| Branch/Item  | `[data-value]`, `[data-path]`, `[data-depth]` | Identity                                      |
| Branch       | `[data-state]`                                | `"open" \| "closed"`                          |
| Item         | `[data-selected]`, `[data-focus]`             | State                                         |
| Branch       | `[data-loading]`                              | Async loading                                 |
| NodeCheckbox | `[data-state]`                                | `"checked" \| "unchecked" \| "indeterminate"` |

### 48.5 Context API

| Property          | Type                        | Description        |
| ----------------- | --------------------------- | ------------------ |
| `expand`          | `(value?) => void`          | Expand nodes       |
| `collapse`        | `(value?) => void`          | Collapse nodes     |
| `select`          | `(value?) => void`          | Select nodes       |
| `deselect`        | `(value?) => void`          | Deselect           |
| `focus`           | `(value) => void`           | Focus node         |
| `toggleChecked`   | `(value, isBranch) => void` | Toggle checkbox    |
| `getVisibleNodes` | `() => VisibleNode<V>[]`    | Flat visible nodes |
| `startRenaming`   | `(value) => void`           | Begin rename       |
| `submitRenaming`  | `(value, label) => void`    | Commit rename      |
| `cancelRenaming`  | `VoidFunction`              | Cancel rename      |

---

## A. Utilities

### A.1 Presence

| Prop                   | Type           | Default | Description             |
| ---------------------- | -------------- | ------- | ----------------------- |
| `present`              | `boolean`      | —       | Controlled presence     |
| `lazyMount`            | `boolean`      | `false` | Lazy mount              |
| `unmountOnExit`        | `boolean`      | `false` | Unmount on exit         |
| `immediate`            | `boolean`      | —       | Sync immediately        |
| `skipAnimationOnMount` | `boolean`      | `false` | Skip initial animation  |
| `onExitComplete`       | `VoidFunction` | —       | Exit animation callback |
| `asChild`              | `boolean`      | —       | Composition             |

### A.2 Highlight

| Prop         | Type                 | Description         |
| ------------ | -------------------- | ------------------- |
| `query`      | `string \| string[]` | Text to highlight   |
| `text`       | `string`             | Source text         |
| `exactMatch` | `boolean`            | Whole word match    |
| `ignoreCase` | `boolean`            | Case insensitive    |
| `matchAll`   | `boolean`            | Match all instances |

### A.3 Format.Number

| Prop                          | Type     | Description                              |
| ----------------------------- | -------- | ---------------------------------------- |
| `value`                       | `number` | Number to format (required)              |
| `...Intl.NumberFormatOptions` | —        | All Intl options (style, currency, etc.) |

### A.4 Format.Byte

| Prop          | Type                            | Description          |
| ------------- | ------------------------------- | -------------------- |
| `value`       | `number`                        | Byte size (required) |
| `unit`        | `'bit' \| 'byte'`               | Unit type            |
| `unitDisplay` | `'long' \| 'short' \| 'narrow'` | Unit display         |
| `unitSystem`  | `'decimal' \| 'binary'`         | 1000 vs 1024         |

### A.5 Format.Time

| Prop          | Type             | Description           |
| ------------- | ---------------- | --------------------- |
| `value`       | `string \| Date` | Time value (required) |
| `withSeconds` | `boolean`        | Include seconds       |
| `amLabel`     | `string`         | Custom AM label       |
| `pmLabel`     | `string`         | Custom PM label       |

---

## B. Collections

### B.1 createListCollection(options)

| Option           | Type                   | Description     |
| ---------------- | ---------------------- | --------------- |
| `items`          | `T[]`                  | Item array      |
| `itemToString`   | `(item: T) => string`  | Display text    |
| `itemToValue`    | `(item: T) => V`       | Value extractor |
| `isItemDisabled` | `(item: T) => boolean` | Disabled check  |

Methods: `find(value)`, `findMany(values)`, `getNextValue(value)`, `getPreviousValue(value)`, `has(value)`, `reorder(from, to)`

Properties: `items`, `firstValue`, `lastValue`

### B.2 createTreeCollection(options)

| Option           | Type                   | Description       |
| ---------------- | ---------------------- | ----------------- |
| `rootNode`       | `T`                    | Root node         |
| `nodeToValue`    | `(node: T) => string`  | Value extractor   |
| `nodeToString`   | `(node: T) => string`  | Display text      |
| `nodeToChildren` | `(node: T) => T[]`     | Children accessor |
| `isNodeDisabled` | `(node: T) => boolean` | Disabled check    |

Navigation: `getFirstNode()`, `getLastNode()`, `getNextNode(value)`, `getPreviousNode(value)`
Hierarchy: `getParentNode(value)`, `getParentNodes(value)`, `getDescendantNodes(value)`
Queries: `isBranchNode(node)`, `getBranchValues()`, `getDepth(value)`
Manipulation: `insertAfter()`, `insertBefore()`, `remove()`, `move()`, `replace()`, `filter()`
Tree collections are immutable.

---

## C. Components in Zag.js but NOT in Ark-UI

The following machines exist in Zag.js but do NOT have corresponding Ark-UI components yet:

1. **Navigation Menu** — Horizontal/vertical nav with submenus, hover triggers, viewport container
2. **Cascade Select** (Beta) — Hierarchical multi-level dropdown selection using TreeCollection
3. **Date Input** (Beta) — Segmented date input field (separate from Date Picker, more like date-field)
4. **Focus Trap** — Utility for trapping focus within a container
5. **Async List** — Utility for async data loading

Note: Zag.js uses the same Slider machine for both single and range variants, which Ark-UI also consolidates. Context Menu is handled via `Menu.ContextTrigger` (not a separate component). Alert Dialog is handled via `Dialog` with `role="alertdialog"`.

---

## D. Components in Our Spec NOT in Ark-UI

These components from our spec list do NOT exist in Ark-UI:

**Input:** `text-field`, `textarea` (use Field.Input / Field.Textarea instead), `range-slider` (use Slider with array value)
**Selection:** `context-menu` (use Menu.ContextTrigger), `autocomplete` (use Combobox)
**Overlay:** `alert-dialog` (use Dialog with role="alertdialog"), `drawer` (not in Ark-UI), `presence` (utility only)
**Navigation:** `breadcrumbs`, `link` (not in Ark-UI)
**Date-Time:** `date-field`, `time-field`, `date-range-field`, `date-range-picker`, `range-calendar` (Date Picker handles range via selectionMode)
**Data Display:** `badge`, `meter`, `skeleton`, `table`, `tag-group`, `grid-list`, `stat` (not in Ark-UI)
**Layout:** `aspect-ratio`, `portal`, `toolbar`, `frame` (not in Ark-UI)
**Specialized:** `color-area`, `color-field`, `color-slider`, `color-swatch`, `color-swatch-picker`, `color-wheel`, `contextual-help` (color sub-components are only within ColorPicker)
**Utility:** `button`, `focus-scope`, `visually-hidden`, `separator`, `as-child` (composition pattern), `ars-provider`, `form`, `live-region` (most are patterns/utilities, not components)

---

## E. Notes on Ark-UI Architecture

1. **asChild pattern**: Every Ark-UI component part supports `asChild` prop for render delegation
2. **RootProvider pattern**: Every component has a `RootProvider` variant for external state management via hooks (`useX()`)
3. **Context pattern**: Every component exposes a `.Context` consumer and `useXContext()` hook
4. **Positioning**: Components with popups use a shared `PositioningOptions` type (placement, offset, flip, etc.)
5. **Layer management**: Overlays use `--layer-index` and `--nested-layer-count` CSS variables
6. **Data scope/part**: All elements use `[data-scope]` and `[data-part]` for styling
7. **Controlled/Uncontrolled**: All state props come in `value`/`defaultValue` pairs
8. **Form integration**: Input components support `name`, `form`, `required`, `invalid`, `disabled`, `readOnly`
9. **i18n**: Many components accept `translations` and `locale` props
10. **Animation**: Components support `lazyMount`, `unmountOnExit`, `skipAnimationOnMount`, `onExitComplete`
