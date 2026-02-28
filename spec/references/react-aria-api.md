---
title: React Aria API Reference
source: https://react-aria.adobe.com/
repository: https://github.com/adobe/react-spectrum
generated: 2026-03-27
total_components: 45+
---

React Aria Component Library — Comprehensive API Reference

> Research compiled from <https://react-aria.adobe.com/> (formerly react-spectrum.adobe.com/react-aria/)
> Source repository: <https://github.com/adobe/react-spectrum> (packages/react-aria-components)

---

## Table of Contents

1. [Buttons & Toggles](#1-buttons--toggles)
2. [Input Components](#2-input-components)
3. [Selection Components](#3-selection-components)
4. [Overlay Components](#4-overlay-components)
5. [Navigation Components](#5-navigation-components)
6. [Date & Time Components](#6-date--time-components)
7. [Data Display Components](#7-data-display-components)
8. [Layout & Utility Components](#8-layout--utility-components)
9. [Color Components](#9-color-components)
10. [Miscellaneous Components](#10-miscellaneous-components)
11. [Components NOT in Our Spec](#11-components-not-in-our-spec)
12. [Shared Patterns Across All Components](#12-shared-patterns-across-all-components)

---

## 1. Buttons & Toggles

### 1.1 Button

**React Aria name:** `Button`
**Category:** Utility / Buttons
**Hook:** `usePress` (underlying interaction hook)

#### 1.1.1 Sub-components

- `Button` — A pressable element supporting mouse, touch, and keyboard interactions.

#### 1.1.2 Props

| Prop                  | Type                                                              | Default               | Description                                                        |
| --------------------- | ----------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------ |
| `isDisabled`          | `boolean`                                                         | —                     | Disables the button                                                |
| `isPending`           | `boolean`                                                         | —                     | Pending state; disables press/hover while maintaining focusability |
| `type`                | `'button' \| 'submit' \| 'reset'`                                 | `'button'`            | Form submission behavior                                           |
| `autoFocus`           | `boolean`                                                         | —                     | Receive focus on render                                            |
| `form`                | `string`                                                          | —                     | Associated form element ID                                         |
| `name`                | `string`                                                          | —                     | Form submission name                                               |
| `value`               | `string`                                                          | —                     | Form submission value                                              |
| `formAction`          | `string \| ((formData: FormData) => void \| Promise<void>)`       | —                     | Form submission URL or handler                                     |
| `formMethod`          | `string`                                                          | —                     | HTTP form submission method                                        |
| `formEncType`         | `string`                                                          | —                     | Form data encoding type                                            |
| `formTarget`          | `string`                                                          | —                     | Form submission target                                             |
| `formNoValidate`      | `boolean`                                                         | —                     | Skip form validation                                               |
| `className`           | `ClassNameOrFunction<ButtonRenderProps>`                          | `'react-aria-Button'` | CSS class name or function                                         |
| `style`               | `CSSProperties \| ((values: ButtonRenderProps) => CSSProperties)` | —                     | Inline styles or computed function                                 |
| `children`            | `ChildrenOrFunction<ButtonRenderProps>`                           | —                     | Content; function receives render state                            |
| `excludeFromTabOrder` | `boolean`                                                         | —                     | Remove from keyboard tab sequence                                  |
| `preventFocusOnPress` | `boolean`                                                         | —                     | Prevent focus movement on press                                    |
| `id`                  | `string`                                                          | —                     | Unique identifier                                                  |
| `slot`                | `string \| null`                                                  | —                     | Slot name for parent composition                                   |
| `render`              | `DOMRenderFunction<'button', ButtonRenderProps>`                  | —                     | Custom render function                                             |
| `aria-label`          | `string`                                                          | —                     | Accessible label                                                   |
| `aria-labelledby`     | `string`                                                          | —                     | Element labeling this button                                       |
| `aria-describedby`    | `string`                                                          | —                     | Element describing this button                                     |
| `aria-details`        | `string`                                                          | —                     | Extended description element                                       |
| `aria-disabled`       | `boolean \| 'true' \| 'false'`                                    | —                     | Accessibility disabled state                                       |
| `aria-pressed`        | `boolean \| 'true' \| 'false' \| 'mixed'`                         | —                     | Toggle button state                                                |
| `aria-expanded`       | `boolean \| 'true' \| 'false'`                                    | —                     | Expansion state                                                    |
| `aria-haspopup`       | `boolean \| 'menu' \| 'listbox' \| 'tree' \| 'grid' \| 'dialog'`  | —                     | Popup availability                                                 |
| `aria-controls`       | `string`                                                          | —                     | Controlled element ID                                              |
| `aria-current`        | `boolean \| 'page' \| 'step' \| 'location' \| 'date' \| 'time'`   | —                     | Current item indicator                                             |

#### 1.1.3 Events

| Event           | Payload         | Description                                 |
| --------------- | --------------- | ------------------------------------------- |
| `onPress`       | `PressEvent`    | Press interaction concludes over the target |
| `onPressStart`  | `PressEvent`    | Press interaction initiates                 |
| `onPressEnd`    | `PressEvent`    | Press interaction concludes                 |
| `onPressUp`     | `PressEvent`    | Press releases over the target              |
| `onPressChange` | `boolean`       | Press state changes                         |
| `onHoverStart`  | `HoverEvent`    | Hovering begins                             |
| `onHoverEnd`    | `HoverEvent`    | Hovering concludes                          |
| `onHoverChange` | `boolean`       | Hover state changes                         |
| `onFocus`       | `FocusEvent`    | Element receives focus                      |
| `onBlur`        | `FocusEvent`    | Element loses focus                         |
| `onFocusChange` | `boolean`       | Focus status changes                        |
| `onKeyDown`     | `KeyboardEvent` | Key is pressed                              |
| `onKeyUp`       | `KeyboardEvent` | Key is released                             |

#### 1.1.4 Render Props (ButtonRenderProps)

| Slot             | Values    | Description                |
| ---------------- | --------- | -------------------------- |
| `isPending`      | `boolean` | Button is in pending state |
| `isPressed`      | `boolean` | Active press state         |
| `isHovered`      | `boolean` | Hover state                |
| `isFocused`      | `boolean` | Focus state                |
| `isFocusVisible` | `boolean` | Keyboard focus visibility  |
| `isDisabled`     | `boolean` | Disabled state             |

#### 1.1.5 Related Types

**PressEvent:** `{ type, pointerType, target, shiftKey, ctrlKey, metaKey, altKey, x, y, key, continuePropagation() }`

---

### 1.2 ToggleButton

**React Aria name:** `ToggleButton`
**Category:** Utility / Buttons

#### 1.2.1 Sub-components

- `ToggleButton` — A button with a selected/unselected state.

#### 1.2.2 Props (in addition to Button props)

| Prop              | Type      | Default | Description                          |
| ----------------- | --------- | ------- | ------------------------------------ |
| `isSelected`      | `boolean` | —       | Controlled selection state           |
| `defaultSelected` | `boolean` | —       | Uncontrolled initial selection state |

#### 1.2.3 Events (in addition to Button events)

| Event      | Payload   | Description             |
| ---------- | --------- | ----------------------- |
| `onChange` | `boolean` | Selection state changes |

#### 1.2.4 Render Props (adds to ButtonRenderProps)

| Slot         | Values    | Description             |
| ------------ | --------- | ----------------------- |
| `isSelected` | `boolean` | Current selection state |

---

### 1.3 ToggleButtonGroup

**React Aria name:** `ToggleButtonGroup`
**Category:** Utility / Buttons

#### 1.3.1 Sub-components

- `ToggleButtonGroup` — Container for a group of toggle buttons with managed selection.
- `ToggleButton` — Individual toggle within the group (uses `id` as key).

#### 1.3.2 Props

| Prop                     | Type                     | Default        | Description                          |
| ------------------------ | ------------------------ | -------------- | ------------------------------------ |
| `selectedKeys`           | `Iterable<Key>`          | —              | Currently selected keys (controlled) |
| `defaultSelectedKeys`    | `Iterable<Key>`          | —              | Initial selected keys (uncontrolled) |
| `selectionMode`          | `'single' \| 'multiple'` | `'single'`     | Single or multiple selection         |
| `disallowEmptySelection` | `boolean`                | —              | Prevent clearing all selections      |
| `isDisabled`             | `boolean`                | —              | Disable all items                    |
| `orientation`            | `Orientation`            | `'horizontal'` | Layout orientation                   |

#### 1.3.3 Events

| Event               | Payload    | Description       |
| ------------------- | ---------- | ----------------- |
| `onSelectionChange` | `Set<Key>` | Selection changes |

---

## 2. Input Components

### 2.1 Checkbox

**React Aria name:** `Checkbox`
**Category:** Input

#### 2.1.1 Sub-components

- `Checkbox` — A checkbox with label, rendered as a `<label>` element.

#### 2.1.2 Props

| Prop                  | Type                                                  | Default    | Description                                      |
| --------------------- | ----------------------------------------------------- | ---------- | ------------------------------------------------ |
| `isSelected`          | `boolean`                                             | —          | Controlled selection state                       |
| `defaultSelected`     | `boolean`                                             | —          | Uncontrolled initial selection                   |
| `isIndeterminate`     | `boolean`                                             | —          | Visual indeterminate state (presentational only) |
| `isDisabled`          | `boolean`                                             | —          | Disables user interaction                        |
| `isReadOnly`          | `boolean`                                             | —          | Can select but not change                        |
| `isRequired`          | `boolean`                                             | —          | Required before form submission                  |
| `isInvalid`           | `boolean`                                             | —          | Marks input as invalid                           |
| `value`               | `string`                                              | —          | Form submission value                            |
| `name`                | `string`                                              | —          | Form submission name                             |
| `form`                | `string`                                              | —          | Associated form element ID                       |
| `inputRef`            | `RefObject<HTMLInputElement>`                         | —          | Reference to underlying input                    |
| `validate`            | `(value: boolean) => ValidationError \| true \| null` | —          | Custom validation                                |
| `validationBehavior`  | `'native' \| 'aria'`                                  | `'native'` | Validation approach                              |
| `autoFocus`           | `boolean`                                             | —          | Focus on render                                  |
| `excludeFromTabOrder` | `boolean`                                             | —          | Remove from tab sequence                         |
| `slot`                | `string \| null`                                      | —          | Slot name                                        |

#### 2.1.3 Events

| Event           | Payload         | Description             |
| --------------- | --------------- | ----------------------- |
| `onChange`      | `boolean`       | Selection state changes |
| `onFocus`       | `FocusEvent`    | Receives focus          |
| `onBlur`        | `FocusEvent`    | Loses focus             |
| `onFocusChange` | `boolean`       | Focus status changes    |
| `onHoverStart`  | `HoverEvent`    | Hover begins            |
| `onHoverEnd`    | `HoverEvent`    | Hover ends              |
| `onHoverChange` | `boolean`       | Hover state changes     |
| `onKeyDown`     | `KeyboardEvent` | Key pressed             |
| `onKeyUp`       | `KeyboardEvent` | Key released            |
| `onPress`       | `PressEvent`    | Press completed         |
| `onPressStart`  | `PressEvent`    | Press begins            |
| `onPressEnd`    | `PressEvent`    | Press ends              |
| `onPressChange` | `boolean`       | Press state changes     |
| `onPressUp`     | `PressEvent`    | Press releases          |

#### 2.1.4 Render Props (CheckboxRenderProps)

| Slot              | Values    | Description               |
| ----------------- | --------- | ------------------------- |
| `isSelected`      | `boolean` | Selection state           |
| `isIndeterminate` | `boolean` | Indeterminate state       |
| `isDisabled`      | `boolean` | Disabled state            |
| `isInvalid`       | `boolean` | Invalid state             |
| `isReadOnly`      | `boolean` | Read-only state           |
| `isPressed`       | `boolean` | Press state               |
| `isFocused`       | `boolean` | Focus state               |
| `isFocusVisible`  | `boolean` | Keyboard focus visibility |
| `isHovered`       | `boolean` | Hover state               |

---

### 2.2 CheckboxGroup

**React Aria name:** `CheckboxGroup`
**Category:** Input

#### 2.2.1 Sub-components

- `CheckboxGroup` — Container for related checkboxes.
- `Label` — Group label.
- `Checkbox` — Individual checkbox items.
- `Text` (slot="description") — Description text.
- `FieldError` — Error message display.

#### 2.2.2 Props

| Prop                 | Type                                                   | Default      | Description                            |
| -------------------- | ------------------------------------------------------ | ------------ | -------------------------------------- |
| `value`              | `string[]`                                             | —            | Current selected values (controlled)   |
| `defaultValue`       | `string[]`                                             | —            | Initial selected values (uncontrolled) |
| `isDisabled`         | `boolean`                                              | —            | Disables the entire group              |
| `isReadOnly`         | `boolean`                                              | —            | Read-only state                        |
| `isRequired`         | `boolean`                                              | —            | Required before form submission        |
| `isInvalid`          | `boolean`                                              | —            | Invalid state                          |
| `name`               | `string`                                               | —            | Form submission name                   |
| `form`               | `string`                                               | —            | Associated form ID                     |
| `validate`           | `(value: string[]) => ValidationError \| true \| null` | —            | Custom validation                      |
| `validationBehavior` | `'native' \| 'aria'`                                   | `'native'`   | Validation approach                    |
| `orientation`        | `Orientation`                                          | `'vertical'` | Checkbox alignment axis                |

#### 2.2.3 Events

| Event           | Payload      | Description          |
| --------------- | ------------ | -------------------- |
| `onChange`      | `string[]`   | Selection changes    |
| `onFocus`       | `FocusEvent` | Group receives focus |
| `onBlur`        | `FocusEvent` | Group loses focus    |
| `onFocusChange` | `boolean`    | Focus status changes |

---

### 2.3 TextField

**React Aria name:** `TextField`
**Category:** Input

#### 2.3.1 Sub-components

- `TextField` — Root container with form field management.
- `Label` — Field label.
- `Input` — Single-line text input element.
- `TextArea` — Multi-line text input element.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message display.

#### 2.3.2 Props

| Prop                  | Type                                                                                  | Default    | Description                |
| --------------------- | ------------------------------------------------------------------------------------- | ---------- | -------------------------- |
| `value`               | `string`                                                                              | —          | Controlled value           |
| `defaultValue`        | `string`                                                                              | —          | Uncontrolled initial value |
| `isDisabled`          | `boolean`                                                                             | —          | Disabled state             |
| `isReadOnly`          | `boolean`                                                                             | —          | Read-only state            |
| `isRequired`          | `boolean`                                                                             | —          | Required field             |
| `isInvalid`           | `boolean`                                                                             | —          | Invalid state              |
| `type`                | `'text' \| 'search' \| 'tel' \| 'url' \| 'email' \| 'password'`                       | `'text'`   | Input type                 |
| `placeholder`         | `string`                                                                              | —          | Placeholder text           |
| `name`                | `string`                                                                              | —          | Form submission name       |
| `minLength`           | `number`                                                                              | —          | Minimum character count    |
| `maxLength`           | `number`                                                                              | —          | Maximum character count    |
| `pattern`             | `string`                                                                              | —          | Regex validation pattern   |
| `autoComplete`        | `string`                                                                              | —          | Autocomplete behavior      |
| `autoCorrect`         | `string`                                                                              | —          | Autocorrect setting        |
| `autoFocus`           | `boolean`                                                                             | —          | Focus on render            |
| `spellCheck`          | `string`                                                                              | —          | Spell-checking             |
| `inputMode`           | `'text' \| 'none' \| 'tel' \| 'url' \| 'email' \| 'numeric' \| 'decimal' \| 'search'` | —          | Virtual keyboard hint      |
| `enterKeyHint`        | `'enter' \| 'done' \| 'go' \| 'next' \| 'previous' \| 'search' \| 'send'`             | —          | Enter key label            |
| `validate`            | `(value: string) => ValidationError \| true \| null`                                  | —          | Custom validation          |
| `validationBehavior`  | `'native' \| 'aria'`                                                                  | `'native'` | Validation approach        |
| `form`                | `string`                                                                              | —          | Associated form ID         |
| `excludeFromTabOrder` | `boolean`                                                                             | —          | Skip tab order             |

#### 2.3.3 Events

| Event                 | Payload            | Description         |
| --------------------- | ------------------ | ------------------- |
| `onChange`            | `string`           | Value changes       |
| `onFocus`             | `FocusEvent`       | Receives focus      |
| `onBlur`              | `FocusEvent`       | Loses focus         |
| `onFocusChange`       | `boolean`          | Focus changes       |
| `onInput`             | `FormEvent`        | Input modified      |
| `onBeforeInput`       | `FormEvent`        | Before modification |
| `onSelect`            | `ReactEvent`       | Text selected       |
| `onCopy`              | `ClipboardEvent`   | Text copied         |
| `onCut`               | `ClipboardEvent`   | Text cut            |
| `onPaste`             | `ClipboardEvent`   | Text pasted         |
| `onCompositionStart`  | `CompositionEvent` | Composition begins  |
| `onCompositionUpdate` | `CompositionEvent` | Composition updates |
| `onCompositionEnd`    | `CompositionEvent` | Composition ends    |
| `onKeyDown`           | `KeyboardEvent`    | Key pressed         |
| `onKeyUp`             | `KeyboardEvent`    | Key released        |

#### 2.3.4 Render Props (TextFieldRenderProps)

| Slot         | Values    | Description     |
| ------------ | --------- | --------------- |
| `isFocused`  | `boolean` | Focus state     |
| `isDisabled` | `boolean` | Disabled state  |
| `isInvalid`  | `boolean` | Invalid state   |
| `isReadOnly` | `boolean` | Read-only state |

---

### 2.4 NumberField

**React Aria name:** `NumberField`
**Category:** Input

#### 2.4.1 Sub-components

- `NumberField` — Root container.
- `Label` — Field label.
- `Group` — Container for input and buttons.
- `Input` — Number input element.
- `Button` (slot="increment") — Increment button.
- `Button` (slot="decrement") — Decrement button.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 2.4.2 Props

| Prop                 | Type                                                 | Default | Description                    |
| -------------------- | ---------------------------------------------------- | ------- | ------------------------------ |
| `value`              | `number`                                             | —       | Controlled value               |
| `defaultValue`       | `number`                                             | —       | Uncontrolled initial value     |
| `minValue`           | `number`                                             | —       | Minimum allowed value          |
| `maxValue`           | `number`                                             | —       | Maximum allowed value          |
| `step`               | `number`                                             | —       | Increment/decrement amount     |
| `isDisabled`         | `boolean`                                            | —       | Disabled state                 |
| `isReadOnly`         | `boolean`                                            | —       | Read-only state                |
| `isRequired`         | `boolean`                                            | —       | Required field                 |
| `isInvalid`          | `boolean`                                            | —       | Invalid state                  |
| `isWheelDisabled`    | `boolean`                                            | —       | Disable scroll value changes   |
| `formatOptions`      | `Intl.NumberFormatOptions`                           | —       | Locale-based number formatting |
| `name`               | `string`                                             | —       | Form submission name           |
| `placeholder`        | `string`                                             | —       | Placeholder text               |
| `autoFocus`          | `boolean`                                            | —       | Auto-focus on render           |
| `incrementAriaLabel` | `string`                                             | —       | Custom increment button label  |
| `decrementAriaLabel` | `string`                                             | —       | Custom decrement button label  |
| `validate`           | `(value: number) => ValidationError \| true \| null` | —       | Custom validation              |
| `validationBehavior` | `'native' \| 'aria'`                                 | —       | Validation approach            |
| `form`               | `string`                                             | —       | Associated form ID             |

#### 2.4.3 Events

| Event           | Payload         | Description    |
| --------------- | --------------- | -------------- |
| `onChange`      | `number`        | Value changes  |
| `onFocus`       | `FocusEvent`    | Receives focus |
| `onBlur`        | `FocusEvent`    | Loses focus    |
| `onFocusChange` | `boolean`       | Focus changes  |
| `onKeyDown`     | `KeyboardEvent` | Key pressed    |
| `onKeyUp`       | `KeyboardEvent` | Key released   |

---

### 2.5 SearchField

**React Aria name:** `SearchField`
**Category:** Input

#### 2.5.1 Sub-components

- `SearchField` — Root container.
- `Label` — Field label.
- `Input` — Text input.
- `Button` — Clear button.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 2.5.2 Props

Same as TextField, plus:

| Prop   | Type     | Default    | Description                     |
| ------ | -------- | ---------- | ------------------------------- |
| `type` | `string` | `'search'` | Input type (defaults to search) |

#### 2.5.3 Events (in addition to TextField events)

| Event      | Payload  | Description          |
| ---------- | -------- | -------------------- |
| `onSubmit` | `string` | Enter key pressed    |
| `onClear`  | `void`   | Clear button pressed |

---

### 2.6 Slider

**React Aria name:** `Slider`
**Category:** Input

#### 2.6.1 Sub-components

- `Slider` — Root container managing slider state.
- `Label` — Slider label.
- `SliderOutput` — Displays formatted current value(s).
- `SliderTrack` — Visual track container.
- `SliderThumb` — Draggable thumb control.

#### 2.6.2 Slider Props

| Prop            | Type                         | Default        | Description                |
| --------------- | ---------------------------- | -------------- | -------------------------- |
| `value`         | `number \| number[]`         | —              | Controlled value           |
| `defaultValue`  | `number \| number[]`         | —              | Uncontrolled initial value |
| `minValue`      | `number`                     | `0`            | Minimum value              |
| `maxValue`      | `number`                     | `100`          | Maximum value              |
| `step`          | `number`                     | `1`            | Step increment             |
| `orientation`   | `'horizontal' \| 'vertical'` | `'horizontal'` | Layout direction           |
| `isDisabled`    | `boolean`                    | —              | Disabled state             |
| `formatOptions` | `Intl.NumberFormatOptions`   | —              | Value display format       |

#### 2.6.3 Slider Events

| Event         | Payload              | Description               |
| ------------- | -------------------- | ------------------------- |
| `onChange`    | `number \| number[]` | Value changes during drag |
| `onChangeEnd` | `number \| number[]` | Thumb released after drag |

#### 2.6.4 SliderThumb Props

| Prop         | Type                          | Default | Description                   |
| ------------ | ----------------------------- | ------- | ----------------------------- |
| `index`      | `number`                      | `0`     | Position in multi-thumb array |
| `name`       | `string`                      | —       | Form submission name          |
| `isDisabled` | `boolean`                     | —       | Individual thumb disable      |
| `inputRef`   | `RefObject<HTMLInputElement>` | —       | Reference to hidden input     |
| `autoFocus`  | `boolean`                     | —       | Focus on mount                |
| `aria-label` | `string`                      | —       | Required for accessibility    |

#### 2.6.5 SliderThumb Events

| Event           | Payload         | Description          |
| --------------- | --------------- | -------------------- |
| `onFocus`       | `FocusEvent`    | Thumb receives focus |
| `onBlur`        | `FocusEvent`    | Thumb loses focus    |
| `onFocusChange` | `boolean`       | Focus changes        |
| `onHoverStart`  | `HoverEvent`    | Hover begins         |
| `onHoverEnd`    | `HoverEvent`    | Hover ends           |
| `onHoverChange` | `boolean`       | Hover changes        |
| `onKeyDown`     | `KeyboardEvent` | Key pressed          |
| `onKeyUp`       | `KeyboardEvent` | Key released         |

#### 2.6.6 Render Props

**SliderTrack:** `{ isDisabled, isDragging, state: { values, getThumbPercent(i), getThumbValueLabel(i) } }`
**SliderThumb:** `{ isDisabled, isDragging, isFocused, isFocusVisible }`

---

### 2.7 Switch

**React Aria name:** `Switch`
**Category:** Input

#### 2.7.1 Sub-components

- `Switch` — A toggle switch rendered as a `<label>` element.

#### 2.7.2 Props

| Prop                  | Type                          | Default | Description                |
| --------------------- | ----------------------------- | ------- | -------------------------- |
| `isSelected`          | `boolean`                     | —       | Controlled selection state |
| `defaultSelected`     | `boolean`                     | —       | Uncontrolled initial state |
| `isDisabled`          | `boolean`                     | —       | Disabled state             |
| `isReadOnly`          | `boolean`                     | —       | Read-only state            |
| `value`               | `string`                      | —       | Form submission value      |
| `name`                | `string`                      | —       | Form submission name       |
| `form`                | `string`                      | —       | Associated form ID         |
| `inputRef`            | `RefObject<HTMLInputElement>` | —       | Reference to input element |
| `autoFocus`           | `boolean`                     | —       | Auto-focus on render       |
| `excludeFromTabOrder` | `boolean`                     | —       | Skip tab order             |

#### 2.7.3 Events

| Event           | Payload         | Description       |
| --------------- | --------------- | ----------------- |
| `onChange`      | `boolean`       | Selection changes |
| `onFocus`       | `FocusEvent`    | Receives focus    |
| `onBlur`        | `FocusEvent`    | Loses focus       |
| `onFocusChange` | `boolean`       | Focus changes     |
| `onHoverStart`  | `HoverEvent`    | Hover begins      |
| `onHoverEnd`    | `HoverEvent`    | Hover ends        |
| `onHoverChange` | `boolean`       | Hover changes     |
| `onKeyDown`     | `KeyboardEvent` | Key pressed       |
| `onKeyUp`       | `KeyboardEvent` | Key released      |

#### 2.7.4 Render Props (SwitchRenderProps)

| Slot             | Values    | Description               |
| ---------------- | --------- | ------------------------- |
| `isSelected`     | `boolean` | Selection state           |
| `isDisabled`     | `boolean` | Disabled state            |
| `isPressed`      | `boolean` | Press state               |
| `isFocusVisible` | `boolean` | Keyboard focus visibility |
| `isReadOnly`     | `boolean` | Read-only state           |
| `isHovered`      | `boolean` | Hover state               |

---

### 2.8 RadioGroup

**React Aria name:** `RadioGroup`
**Category:** Input

#### 2.8.1 Sub-components

- `RadioGroup` — Container for mutually exclusive radio options.
- `Radio` — Individual radio option.
- `Label` — Group label.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 2.8.2 RadioGroup Props

| Prop                 | Type                                                 | Default      | Description                      |
| -------------------- | ---------------------------------------------------- | ------------ | -------------------------------- |
| `value`              | `string \| null`                                     | —            | Selected value (controlled)      |
| `defaultValue`       | `string \| null`                                     | —            | Initial selection (uncontrolled) |
| `isDisabled`         | `boolean`                                            | —            | Disables all options             |
| `isReadOnly`         | `boolean`                                            | —            | Read-only state                  |
| `isRequired`         | `boolean`                                            | —            | Required field                   |
| `isInvalid`          | `boolean`                                            | —            | Invalid state                    |
| `name`               | `string`                                             | —            | Form submission name             |
| `orientation`        | `Orientation`                                        | `'vertical'` | Layout axis                      |
| `validate`           | `(value: string) => ValidationError \| true \| null` | —            | Custom validation                |
| `validationBehavior` | `'native' \| 'aria'`                                 | `'native'`   | Validation approach              |

#### 2.8.3 RadioGroup Events

| Event           | Payload      | Description          |
| --------------- | ------------ | -------------------- |
| `onChange`      | `string`     | Selection changes    |
| `onFocus`       | `FocusEvent` | Group receives focus |
| `onBlur`        | `FocusEvent` | Group loses focus    |
| `onFocusChange` | `boolean`    | Focus changes        |

#### 2.8.4 Radio Props

| Prop         | Type                          | Default      | Description                |
| ------------ | ----------------------------- | ------------ | -------------------------- |
| `value`      | `string`                      | **required** | Submission value           |
| `isDisabled` | `boolean`                     | —            | Individual option disabled |
| `autoFocus`  | `boolean`                     | —            | Focus on render            |
| `inputRef`   | `RefObject<HTMLInputElement>` | —            | Reference to input         |

#### 2.8.5 Radio Events

Same as Checkbox events (press, hover, focus, keyboard events).

#### 2.8.6 Radio Render Props

`{ isSelected, isDisabled, isFocused, isPressed, isFocusVisible, isHovered, isReadOnly, isInvalid }`

---

### 2.9 FileTrigger

**React Aria name:** `FileTrigger`
**Category:** Input

#### 2.9.1 Sub-components

- `FileTrigger` — Wrapper that enables file selection through a pressable child.

#### 2.9.2 Props

| Prop                | Type                      | Default | Description                            |
| ------------------- | ------------------------- | ------- | -------------------------------------- |
| `acceptDirectory`   | `boolean`                 | —       | Allows directory selection             |
| `acceptedFileTypes` | `readonly string[]`       | —       | File type constraints by MIME type     |
| `allowsMultiple`    | `boolean`                 | —       | Permits multiple file selection        |
| `defaultCamera`     | `'user' \| 'environment'` | —       | Media capture mode                     |
| `children`          | `ReactNode`               | —       | Trigger component (Button, Link, etc.) |

#### 2.9.3 Events

| Event      | Payload            | Description        |
| ---------- | ------------------ | ------------------ |
| `onSelect` | `FileList \| null` | User selects files |

---

## 3. Selection Components

### 3.1 ComboBox

**React Aria name:** `ComboBox`
**Category:** Selection

#### 3.1.1 Sub-components

- `ComboBox` — Root wrapper combining input and listbox.
- `Label` — Field label.
- `Input` — Text input for filtering.
- `Button` — Trigger button (chevron icon).
- `Popover` — Menu container overlay.
- `ListBox` / `ComboBoxListBox` — Underlying list.
- `ListBoxItem` / `ComboBoxItem` — Individual items.
- `ComboBoxSection` — Groups items into sections.
- `ComboBoxValue` — Displays selected items.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 3.1.2 ComboBox Props

| Prop                    | Type                                         | Default    | Description                      |
| ----------------------- | -------------------------------------------- | ---------- | -------------------------------- |
| `allowsCustomValue`     | `boolean`                                    | —          | Allow non-item matching values   |
| `allowsEmptyCollection` | `boolean`                                    | —          | Keep menu open when empty        |
| `defaultFilter`         | `(textValue, inputValue) => boolean`         | —          | Custom filter function           |
| `defaultInputValue`     | `string`                                     | —          | Initial input (uncontrolled)     |
| `defaultItems`          | `Iterable<T>`                                | —          | Items list (uncontrolled)        |
| `defaultSelectedKey`    | `Key \| null`                                | —          | Initial selection (uncontrolled) |
| `disabledKeys`          | `Iterable<Key>`                              | —          | Non-interactive item keys        |
| `formValue`             | `'text' \| 'key'`                            | `'key'`    | Submit text or key value         |
| `inputValue`            | `string`                                     | —          | Input value (controlled)         |
| `isDisabled`            | `boolean`                                    | —          | Disabled state                   |
| `isInvalid`             | `boolean`                                    | —          | Invalid state                    |
| `isReadOnly`            | `boolean`                                    | —          | Read-only state                  |
| `isRequired`            | `boolean`                                    | —          | Required field                   |
| `items`                 | `Iterable<T>`                                | —          | Items list (controlled)          |
| `menuTrigger`           | `'input' \| 'focus' \| 'manual'`             | `'input'`  | Menu open behavior               |
| `name`                  | `string`                                     | —          | Form submission name             |
| `placeholder`           | `string`                                     | —          | Placeholder text                 |
| `selectedKey`           | `Key \| null`                                | —          | Selected key (controlled)        |
| `selectionMode`         | `'single' \| 'multiple'`                     | `'single'` | Selection mode                   |
| `shouldFocusWrap`       | `boolean`                                    | —          | Circular keyboard navigation     |
| `validate`              | `(value) => ValidationError \| true \| null` | —          | Custom validation                |
| `validationBehavior`    | `'native' \| 'aria'`                         | `'native'` | Validation approach              |

#### 3.1.3 ComboBox Events

| Event               | Payload                               | Description            |
| ------------------- | ------------------------------------- | ---------------------- |
| `onChange`          | `value`                               | Selected value changes |
| `onInputChange`     | `string`                              | Input text changes     |
| `onSelectionChange` | `Key`                                 | Selection changes      |
| `onOpenChange`      | `(isOpen: boolean, trigger?: string)` | Menu opens/closes      |
| `onFocus`           | `FocusEvent`                          | Input receives focus   |
| `onBlur`            | `FocusEvent`                          | Input loses focus      |
| `onFocusChange`     | `boolean`                             | Focus changes          |
| `onKeyDown`         | `KeyboardEvent`                       | Key pressed            |
| `onKeyUp`           | `KeyboardEvent`                       | Key released           |

---

### 3.2 ListBox

**React Aria name:** `ListBox`
**Category:** Selection / Data Display

#### 3.2.1 Sub-components

- `ListBox` — Container for selectable items.
- `ListBoxItem` — Individual selectable option.
- `ListBoxSection` — Groups related items with optional header.
- `Header` — Section header.
- `ListBoxLoadMoreItem` — Infinite scroll loading trigger.
- `Text` (slot="label") — Primary item text.
- `Text` (slot="description") — Secondary item text.

#### 3.2.2 ListBox Props

| Prop                     | Type                         | Default            | Description                 |
| ------------------------ | ---------------------------- | ------------------ | --------------------------- |
| `selectionMode`          | `'single' \| 'multiple'`     | —                  | Selection behavior          |
| `selectedKeys`           | `Iterable<Key> \| 'all'`     | —                  | Controlled selection        |
| `defaultSelectedKeys`    | `Iterable<Key> \| 'all'`     | —                  | Initial selection           |
| `items`                  | `Iterable<T>`                | —                  | Dynamic collection data     |
| `layout`                 | `'stack' \| 'grid'`          | `'stack'`          | Item arrangement            |
| `orientation`            | `'vertical' \| 'horizontal'` | `'vertical'`       | Layout direction            |
| `disabledKeys`           | `Iterable<Key>`              | —                  | Unselectable items          |
| `disallowEmptySelection` | `boolean`                    | —                  | Prevent clearing selections |
| `escapeKeyBehavior`      | `'none' \| 'clearSelection'` | `'clearSelection'` | Escape key handling         |
| `shouldFocusOnHover`     | `boolean`                    | —                  | Auto-focus on hover         |
| `shouldFocusWrap`        | `boolean`                    | —                  | Focus wraps at boundaries   |
| `shouldSelectOnPressUp`  | `boolean`                    | —                  | Selection timing            |
| `autoFocus`              | `boolean`                    | —                  | Auto-focus behavior         |
| `renderEmptyState`       | `() => ReactNode`            | —                  | Custom empty state          |
| `dragAndDropHooks`       | `DragAndDropHooks`           | —                  | DND support                 |

#### 3.2.3 ListBox Events

| Event               | Payload     | Description       |
| ------------------- | ----------- | ----------------- |
| `onSelectionChange` | `Selection` | Selection changes |
| `onAction`          | `Key`       | Item activated    |

#### 3.2.4 ListBoxItem Props

| Prop         | Type      | Default | Description        |
| ------------ | --------- | ------- | ------------------ |
| `id`         | `Key`     | —       | Unique identifier  |
| `textValue`  | `string`  | —       | Text for typeahead |
| `isDisabled` | `boolean` | —       | Disable selection  |
| `href`       | `string`  | —       | Make item a link   |
| `target`     | `string`  | —       | Link target        |

#### 3.2.5 ListBoxItem Events

Press, hover, focus, keyboard events (same as Button).

#### 3.2.6 ListBoxItem Render Props

`{ isSelected, isFocused, isFocusVisible, isPressed, isHovered, isDisabled, selectionMode, selectionBehavior }`

#### 3.2.7 ListBoxLoadMoreItem Props

| Prop           | Type         | Default | Description                  |
| -------------- | ------------ | ------- | ---------------------------- |
| `isLoading`    | `boolean`    | —       | Show/hide spinner            |
| `scrollOffset` | `number`     | `1`     | Trigger distance from bottom |
| `onLoadMore`   | `() => void` | —       | Fetch callback               |

---

### 3.3 Menu

**React Aria name:** `Menu` + `MenuTrigger`
**Category:** Selection

#### 3.3.1 Sub-components

- `MenuTrigger` — Opens a menu on user interaction.
- `Menu` — Container for menu items.
- `MenuItem` — Individual selectable option.
- `MenuSection` — Groups related items with optional header.
- `SubmenuTrigger` — Wrapper for nested submenu activation.
- `Separator` — Visual divider.
- `Header` — Section header.
- `Keyboard` — Keyboard shortcut display.
- `Popover` — Menu container overlay.

#### 3.3.2 MenuTrigger Props

| Prop          | Type                     | Default   | Description                     |
| ------------- | ------------------------ | --------- | ------------------------------- |
| `isOpen`      | `boolean`                | —         | Controlled open state           |
| `defaultOpen` | `boolean`                | —         | Uncontrolled initial open state |
| `trigger`     | `'press' \| 'longPress'` | `'press'` | Activation method               |

#### 3.3.3 MenuTrigger Events

| Event          | Payload   | Description        |
| -------------- | --------- | ------------------ |
| `onOpenChange` | `boolean` | Open state changes |

#### 3.3.4 Menu Props

| Prop                     | Type                               | Default            | Description             |
| ------------------------ | ---------------------------------- | ------------------ | ----------------------- |
| `items`                  | `Iterable<T>`                      | —                  | Dynamic collection data |
| `selectionMode`          | `'none' \| 'single' \| 'multiple'` | —                  | Selection mode          |
| `selectedKeys`           | `Iterable<Key> \| 'all'`           | —                  | Controlled selection    |
| `defaultSelectedKeys`    | `Iterable<Key> \| 'all'`           | —                  | Initial selection       |
| `disabledKeys`           | `Iterable<Key>`                    | —                  | Disabled items          |
| `disallowEmptySelection` | `boolean`                          | —                  | Prevent empty selection |
| `shouldFocusWrap`        | `boolean`                          | —                  | Circular navigation     |
| `shouldCloseOnSelect`    | `boolean`                          | —                  | Auto-close on select    |
| `autoFocus`              | `boolean \| FocusStrategy`         | —                  | Initial focus           |
| `escapeKeyBehavior`      | `'none' \| 'clearSelection'`       | `'clearSelection'` | Escape handling         |
| `renderEmptyState`       | `() => ReactNode`                  | —                  | Empty state content     |

#### 3.3.5 Menu Events

| Event               | Payload     | Description       |
| ------------------- | ----------- | ----------------- |
| `onAction`          | `Key`       | Item selected     |
| `onSelectionChange` | `Selection` | Selection changes |
| `onClose`           | `void`      | Menu closes       |

#### 3.3.6 MenuItem Props

| Prop                  | Type      | Default | Description             |
| --------------------- | --------- | ------- | ----------------------- |
| `id`                  | `Key`     | —       | Unique identifier       |
| `textValue`           | `string`  | —       | Text for typeahead      |
| `isDisabled`          | `boolean` | —       | Disabled state          |
| `href`                | `string`  | —       | Link destination        |
| `target`              | `string`  | —       | Link target             |
| `shouldCloseOnSelect` | `boolean` | —       | Override close behavior |

#### 3.3.7 MenuItem Events

| Event           | Payload      | Description     |
| --------------- | ------------ | --------------- |
| `onAction`      | `void`       | Item action     |
| `onPress`       | `PressEvent` | Press completed |
| `onPressStart`  | `PressEvent` | Press begins    |
| `onPressEnd`    | `PressEvent` | Press ends      |
| `onHoverStart`  | `HoverEvent` | Hover begins    |
| `onHoverEnd`    | `HoverEvent` | Hover ends      |
| `onHoverChange` | `boolean`    | Hover changes   |
| `onFocus`       | `FocusEvent` | Receives focus  |
| `onBlur`        | `FocusEvent` | Loses focus     |
| `onFocusChange` | `boolean`    | Focus changes   |

#### 3.3.8 MenuItem Render Props

`{ isSelected, selectionMode, isDisabled, isFocused, isFocusVisible, hasSubmenu, isHovered, isPressed, isOpen }`

#### 3.3.9 SubmenuTrigger Props

| Prop    | Type     | Default | Description       |
| ------- | -------- | ------- | ----------------- |
| `delay` | `number` | `200`   | Hover delay in ms |

#### 3.3.10 MenuSection Props

| Prop                     | Type                               | Default | Description                      |
| ------------------------ | ---------------------------------- | ------- | -------------------------------- |
| `id`                     | `Key`                              | —       | Section identifier               |
| `items`                  | `Iterable<T>`                      | —       | Dynamic items                    |
| `selectionMode`          | `'none' \| 'single' \| 'multiple'` | —       | Per-section selection            |
| `selectedKeys`           | `Iterable<Key> \| 'all'`           | —       | Section selection (controlled)   |
| `defaultSelectedKeys`    | `Iterable<Key> \| 'all'`           | —       | Section selection (uncontrolled) |
| `disabledKeys`           | `Iterable<Key>`                    | —       | Disabled items                   |
| `disallowEmptySelection` | `boolean`                          | —       | Prevent empty selection          |
| `shouldCloseOnSelect`    | `boolean`                          | —       | Close on select                  |

---

### 3.4 Select

**React Aria name:** `Select`
**Category:** Selection

#### 3.4.1 Sub-components

- `Select` — Root wrapper managing selection state.
- `Label` — Field label.
- `Button` — Trigger element.
- `SelectValue` — Displays currently selected item.
- `Popover` — Dropdown container.
- `ListBox` — List container.
- `ListBoxItem` — Individual item.
- `ListBoxSection` — Item section.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 3.4.2 Select Props

| Prop                    | Type                                         | Default    | Description                |
| ----------------------- | -------------------------------------------- | ---------- | -------------------------- |
| `selectedKey`           | `Key \| null`                                | —          | Controlled key selection   |
| `defaultSelectedKey`    | `Key \| null`                                | —          | Initial key (uncontrolled) |
| `selectionMode`         | `'single' \| 'multiple'`                     | `'single'` | Selection mode             |
| `isDisabled`            | `boolean`                                    | —          | Disabled state             |
| `isRequired`            | `boolean`                                    | —          | Required field             |
| `isInvalid`             | `boolean`                                    | —          | Invalid state              |
| `isOpen`                | `boolean`                                    | —          | Controlled menu visibility |
| `defaultOpen`           | `boolean`                                    | —          | Initial open state         |
| `allowsEmptyCollection` | `boolean`                                    | —          | Open with no items         |
| `disabledKeys`          | `Iterable<Key>`                              | —          | Disabled items             |
| `items`                 | `Iterable<T>`                                | —          | Dynamic data               |
| `name`                  | `string`                                     | —          | Form submission name       |
| `autoFocus`             | `boolean`                                    | —          | Focus on mount             |
| `form`                  | `string`                                     | —          | Associated form ID         |
| `placeholder`           | `string`                                     | —          | Default display text       |
| `autoComplete`          | `string`                                     | —          | Autocomplete hint          |
| `validate`              | `(value) => ValidationError \| true \| null` | —          | Custom validation          |
| `validationBehavior`    | `'native' \| 'aria'`                         | `'native'` | Validation approach        |

#### 3.4.3 Select Events

| Event               | Payload         | Description           |
| ------------------- | --------------- | --------------------- |
| `onChange`          | `value`         | Selection changes     |
| `onSelectionChange` | `Key`           | Key selection changes |
| `onOpenChange`      | `boolean`       | Menu opens/closes     |
| `onFocus`           | `FocusEvent`    | Receives focus        |
| `onBlur`            | `FocusEvent`    | Loses focus           |
| `onFocusChange`     | `boolean`       | Focus changes         |
| `onKeyDown`         | `KeyboardEvent` | Key pressed           |
| `onKeyUp`           | `KeyboardEvent` | Key released          |

#### 3.4.4 SelectValue Props

| Prop          | Type        | Default | Description                   |
| ------------- | ----------- | ------- | ----------------------------- |
| `placeholder` | `ReactNode` | —       | Content when nothing selected |

#### 3.4.5 SelectValue Render Props

`{ selectedText, defaultChildren, selectedItems, state }`

---

### 3.5 TagGroup

**React Aria name:** `TagGroup`
**Category:** Selection

#### 3.5.1 Sub-components

- `TagGroup` — Root container.
- `TagList` — List container for tags.
- `Tag` — Individual tag item.
- `Label` — Group label.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 3.5.2 TagGroup Props

| Prop                     | Type                         | Default            | Description          |
| ------------------------ | ---------------------------- | ------------------ | -------------------- |
| `selectionMode`          | `SelectionMode`              | —                  | Selection behavior   |
| `selectedKeys`           | `Iterable<Key> \| 'all'`     | —                  | Controlled selection |
| `defaultSelectedKeys`    | `Iterable<Key> \| 'all'`     | —                  | Initial selection    |
| `disabledKeys`           | `Iterable<Key>`              | —                  | Disabled items       |
| `disallowEmptySelection` | `boolean`                    | —                  | Prevent clearing     |
| `escapeKeyBehavior`      | `'none' \| 'clearSelection'` | `'clearSelection'` | Escape handling      |

#### 3.5.3 TagGroup Events

| Event               | Payload     | Description       |
| ------------------- | ----------- | ----------------- |
| `onSelectionChange` | `Selection` | Selection changes |
| `onRemove`          | `Set<Key>`  | Tag(s) removed    |

#### 3.5.4 Tag Props

| Prop         | Type      | Default | Description        |
| ------------ | --------- | ------- | ------------------ |
| `id`         | `Key`     | —       | Unique identifier  |
| `textValue`  | `string`  | —       | Accessibility text |
| `isDisabled` | `boolean` | —       | Disabled state     |
| `href`       | `string`  | —       | Link URL           |

#### 3.5.5 Tag Events

Press, hover, focus events (same as Button).

#### 3.5.6 Tag Render Props

`{ allowsRemoving, isSelected, isDisabled, isFocused, isFocusVisible, isHovered, isPressed }`

---

## 4. Overlay Components

### 4.1 Dialog (within Modal)

**React Aria name:** `Dialog`
**Category:** Overlay
**Note:** Dialog does not have its own page; documented within Modal page.

#### 4.1.1 Sub-components

- `DialogTrigger` — Manages open/close state for dialog overlays.
- `Dialog` — The dialog content container.
- `Modal` — Alternative to ModalOverlay (no backdrop styling).
- `ModalOverlay` — Backdrop + modal container.
- `Heading` — Dialog title (slot="title").
- `Button` (slot="close") — Close button.

#### 4.1.2 DialogTrigger Props

| Prop          | Type      | Default | Description                |
| ------------- | --------- | ------- | -------------------------- |
| `isOpen`      | `boolean` | —       | Controlled open state      |
| `defaultOpen` | `boolean` | —       | Uncontrolled initial state |

#### 4.1.3 DialogTrigger Events

| Event          | Payload   | Description        |
| -------------- | --------- | ------------------ |
| `onOpenChange` | `boolean` | Open state changes |

#### 4.1.4 Dialog Props

| Prop        | Type                        | Default               | Description        |
| ----------- | --------------------------- | --------------------- | ------------------ |
| `role`      | `'dialog' \| 'alertdialog'` | `'dialog'`            | Accessibility role |
| `className` | `string`                    | `'react-aria-Dialog'` | CSS class          |

#### 4.1.5 ModalOverlay Props

| Prop                           | Type                            | Default         | Description                  |
| ------------------------------ | ------------------------------- | --------------- | ---------------------------- |
| `isOpen`                       | `boolean`                       | —               | Controlled open state        |
| `defaultOpen`                  | `boolean`                       | —               | Uncontrolled initial state   |
| `isDismissable`                | `boolean`                       | `false`         | Close on outside interaction |
| `isKeyboardDismissDisabled`    | `boolean`                       | `false`         | Disable Escape key close     |
| `shouldCloseOnInteractOutside` | `(element: Element) => boolean` | —               | Custom close logic           |
| `UNSTABLE_portalContainer`     | `Element`                       | `document.body` | Portal target                |

#### 4.1.6 ModalOverlay Events

| Event          | Payload   | Description        |
| -------------- | --------- | ------------------ |
| `onOpenChange` | `boolean` | Open state changes |

#### 4.1.7 ModalOverlay Render Props

`{ isEntering, isExiting }`

---

### 4.2 Popover

**React Aria name:** `Popover`
**Category:** Overlay

#### 4.2.1 Sub-components

- `Popover` — Positioned overlay element.
- `OverlayArrow` — Directional arrow pointing to trigger.

#### 4.2.2 Popover Props

| Prop                           | Type                            | Default         | Description                 |
| ------------------------------ | ------------------------------- | --------------- | --------------------------- |
| `placement`                    | `Placement`                     | `'bottom'`      | Position relative to anchor |
| `isOpen`                       | `boolean`                       | —               | Controlled open state       |
| `defaultOpen`                  | `boolean`                       | —               | Uncontrolled initial state  |
| `triggerRef`                   | `RefObject<Element>`            | —               | Custom anchor element       |
| `offset`                       | `number`                        | `8`             | Main axis offset            |
| `crossOffset`                  | `number`                        | `0`             | Cross axis offset           |
| `shouldFlip`                   | `boolean`                       | `true`          | Flip if insufficient space  |
| `shouldUpdatePosition`         | `boolean`                       | `true`          | Auto-update on scroll       |
| `maxHeight`                    | `number`                        | —               | Maximum height              |
| `containerPadding`             | `number`                        | `12`            | Boundary padding            |
| `arrowBoundaryOffset`          | `number`                        | `0`             | Arrow edge margin           |
| `arrowRef`                     | `RefObject<Element>`            | —               | Arrow element ref           |
| `boundaryElement`              | `Element`                       | `document.body` | Positioning boundary        |
| `scrollRef`                    | `RefObject<Element>`            | —               | Scrollable region           |
| `isKeyboardDismissDisabled`    | `boolean`                       | `false`         | Disable Escape close        |
| `isNonModal`                   | `boolean`                       | —               | Allow outside interaction   |
| `shouldCloseOnInteractOutside` | `(element: Element) => boolean` | —               | Custom close logic          |
| `UNSTABLE_portalContainer`     | `Element`                       | `document.body` | Portal container            |
| `trigger`                      | `string`                        | —               | Trigger source identifier   |

#### 4.2.3 Popover Events

| Event          | Payload   | Description        |
| -------------- | --------- | ------------------ |
| `onOpenChange` | `boolean` | Open state changes |

#### 4.2.4 Popover Render Props

`{ isEntering, isExiting, placement }`

#### 4.2.5 OverlayArrow Render Props

`{ placement }`

---

### 4.3 Tooltip

**React Aria name:** `Tooltip` + `TooltipTrigger`
**Category:** Overlay

#### 4.3.1 Sub-components

- `TooltipTrigger` — Wrapper managing tooltip visibility.
- `Tooltip` — The tooltip overlay.
- `OverlayArrow` — Arrow pointing to trigger.

#### 4.3.2 TooltipTrigger Props

| Prop          | Type      | Default | Description                    |
| ------------- | --------- | ------- | ------------------------------ |
| `isOpen`      | `boolean` | —       | Controlled open state          |
| `defaultOpen` | `boolean` | —       | Uncontrolled state             |
| `delay`       | `number`  | `1500`  | Show delay (ms)                |
| `closeDelay`  | `number`  | `500`   | Hide delay (ms)                |
| `trigger`     | `'focus'` | —       | Only show on focus (not hover) |
| `isDisabled`  | `boolean` | —       | Disable tooltip                |

#### 4.3.3 TooltipTrigger Events

| Event          | Payload   | Description        |
| -------------- | --------- | ------------------ |
| `onOpenChange` | `boolean` | Open state changes |

#### 4.3.4 Tooltip Props

| Prop                        | Type                 | Default         | Description                  |
| --------------------------- | -------------------- | --------------- | ---------------------------- |
| `placement`                 | `Placement`          | `'top'`         | Position relative to trigger |
| `offset`                    | `number`             | `0`             | Main axis offset             |
| `crossOffset`               | `number`             | `0`             | Cross axis offset            |
| `containerPadding`          | `number`             | `12`            | Boundary padding             |
| `shouldFlip`                | `boolean`            | `true`          | Auto-flip                    |
| `arrowBoundaryOffset`       | `number`             | `0`             | Arrow margin                 |
| `isOpen`                    | `boolean`            | —               | Controlled state             |
| `defaultOpen`               | `boolean`            | —               | Uncontrolled state           |
| `triggerRef`                | `RefObject<Element>` | —               | Anchor element               |
| `isKeyboardDismissDisabled` | `boolean`            | `false`         | Disable Escape close         |
| `UNSTABLE_portalContainer`  | `Element`            | `document.body` | Portal container             |

#### 4.3.5 Tooltip Events

| Event          | Payload   | Description        |
| -------------- | --------- | ------------------ |
| `onOpenChange` | `boolean` | Open state changes |

#### 4.3.6 Tooltip Render Props

`{ isEntering, isExiting }`

**Key behavior:** Tooltips are NOT shown on touch screen interactions. Supports warmup/cooldown delay model.

---

### 4.4 Toast

**React Aria name:** `ToastRegion` + `Toast` + `ToastQueue`
**Category:** Overlay

#### 4.4.1 Sub-components

- `ToastRegion` — Landmark region container at app root.
- `Toast` — Individual toast notification.
- `ToastContent` — Structured content container.
- `Text` (slot="title") — Toast title.
- `Text` (slot="description") — Toast description.
- `Button` (slot="close") — Close button.

#### 4.4.2 ToastQueue (state manager, outside React)

| Method  | Signature                                                  | Description            |
| ------- | ---------------------------------------------------------- | ---------------------- |
| `add`   | `(content: T, options?: { timeout?, onClose? }) => string` | Add toast, returns key |
| `close` | `(key: string) => void`                                    | Dismiss toast          |

Constructor: `new ToastQueue({ wrapUpdate? })`

#### 4.4.3 ToastRegion Props

| Prop    | Type            | Default      | Description          |
| ------- | --------------- | ------------ | -------------------- |
| `queue` | `ToastQueue<T>` | **required** | Toast queue instance |

**Navigation:** F6 (forward), Shift+F6 (backward) for keyboard users.

---

## 5. Navigation Components

### 5.1 Breadcrumbs

**React Aria name:** `Breadcrumbs`
**Category:** Navigation

#### 5.1.1 Sub-components

- `Breadcrumbs` — Container rendering ordered breadcrumb list.
- `Breadcrumb` — Individual breadcrumb item.
- `Link` — Navigation link within breadcrumb.

#### 5.1.2 Breadcrumbs Props

| Prop         | Type          | Default | Description             |
| ------------ | ------------- | ------- | ----------------------- |
| `items`      | `Iterable<T>` | —       | Dynamic items           |
| `isDisabled` | `boolean`     | —       | Disable all breadcrumbs |

#### 5.1.3 Breadcrumbs Events

| Event      | Payload | Description        |
| ---------- | ------- | ------------------ |
| `onAction` | `Key`   | Breadcrumb clicked |

#### 5.1.4 Breadcrumb Render Props

`{ isCurrent }`

---

### 5.2 Link

**React Aria name:** `Link`
**Category:** Navigation

#### 5.2.1 Props

| Prop             | Type                | Default | Description               |
| ---------------- | ------------------- | ------- | ------------------------- |
| `href`           | `string`            | —       | Link URL                  |
| `download`       | `string \| boolean` | —       | Download hint             |
| `hrefLang`       | `string`            | —       | Linked resource language  |
| `ping`           | `string`            | —       | Ping URLs                 |
| `referrerPolicy` | `string`            | —       | Referrer policy           |
| `rel`            | `string`            | —       | Relationship              |
| `target`         | `string`            | —       | Target window             |
| `isDisabled`     | `boolean`           | —       | Disabled state            |
| `autoFocus`      | `boolean`           | —       | Focus on render           |
| `routerOptions`  | `any`               | —       | Client-side router config |

#### 5.2.2 Events

Same as Button (press, hover, focus, keyboard events).

#### 5.2.3 Render Props

`{ isHovered, isPressed, isFocused, isFocusVisible, isDisabled }`

---

### 5.3 Tabs

**React Aria name:** `Tabs`
**Category:** Navigation

#### 5.3.1 Sub-components

- `Tabs` — Root wrapper.
- `TabList` — Container for tab triggers.
- `Tab` — Individual tab trigger.
- `TabPanels` — Container for tab panel content sections.
- `TabPanel` — Individual content section.

#### 5.3.2 Tabs Props

| Prop                 | Type                         | Default        | Description                      |
| -------------------- | ---------------------------- | -------------- | -------------------------------- |
| `selectedKey`        | `Key`                        | —              | Controlled selected tab          |
| `defaultSelectedKey` | `Key`                        | —              | Initial selection (uncontrolled) |
| `orientation`        | `'horizontal' \| 'vertical'` | `'horizontal'` | Layout direction                 |
| `keyboardActivation` | `'manual' \| 'automatic'`    | `'automatic'`  | Activation behavior              |
| `isDisabled`         | `boolean`                    | —              | Disable all tabs                 |
| `disabledKeys`       | `Iterable<Key>`              | —              | Specific disabled tabs           |

#### 5.3.3 Tabs Events

| Event               | Payload | Description       |
| ------------------- | ------- | ----------------- |
| `onSelectionChange` | `Key`   | Selection changes |

#### 5.3.4 Tab Props

| Prop         | Type      | Default | Description               |
| ------------ | --------- | ------- | ------------------------- |
| `id`         | `Key`     | —       | Unique identifier         |
| `isDisabled` | `boolean` | —       | Disabled state            |
| `href`       | `string`  | —       | Optional link destination |

#### 5.3.5 Tab Events

Press, hover, focus events.

#### 5.3.6 Tab Render Props

`{ isSelected, isDisabled, isFocused, isFocusVisible, isHovered, isPressed }`

#### 5.3.7 TabPanel Props

| Prop               | Type      | Default | Description                  |
| ------------------ | --------- | ------- | ---------------------------- |
| `id`               | `Key`     | —       | Matches Tab id               |
| `shouldForceMount` | `boolean` | `false` | Mount inactive panels in DOM |

#### 5.3.8 TabPanel Render Props

`{ isSelected, isDisabled, isFocusVisible }`

---

## 6. Date & Time Components

### 6.1 Calendar

**React Aria name:** `Calendar`
**Category:** Date-Time

#### 6.1.1 Sub-components

- `Calendar` — Root component.
- `CalendarGrid` — Table for date cells.
- `CalendarGridHeader` — Weekday header row.
- `CalendarHeaderCell` — Individual weekday cell.
- `CalendarGridBody` — Date cell container.
- `CalendarCell` — Individual date cell.
- `Heading` — Month/year heading.
- `Button` (previous/next) — Navigation.

#### 6.1.2 Calendar Props

| Prop                  | Type                                                          | Default         | Description                 |
| --------------------- | ------------------------------------------------------------- | --------------- | --------------------------- |
| `value`               | `DateValue`                                                   | —               | Controlled selected date    |
| `defaultValue`        | `DateValue`                                                   | —               | Initial date (uncontrolled) |
| `focusedValue`        | `DateValue`                                                   | —               | Controlled focused date     |
| `defaultFocusedValue` | `DateValue`                                                   | —               | Initial focused date        |
| `minValue`            | `DateValue`                                                   | —               | Minimum selectable date     |
| `maxValue`            | `DateValue`                                                   | —               | Maximum selectable date     |
| `isDateUnavailable`   | `(date: DateValue) => boolean`                                | —               | Disable specific dates      |
| `isInvalid`           | `boolean`                                                     | —               | Invalid state               |
| `isDisabled`          | `boolean`                                                     | —               | Disabled state              |
| `isReadOnly`          | `boolean`                                                     | —               | Read-only state             |
| `firstDayOfWeek`      | `'sun' \| 'mon' \| 'tue' \| 'wed' \| 'thu' \| 'fri' \| 'sat'` | —               | Override locale week start  |
| `visibleDuration`     | `{ months: number }`                                          | `{ months: 1 }` | Months displayed            |
| `pageBehavior`        | `'visible' \| 'single'`                                       | `'visible'`     | Navigation behavior         |
| `createCalendar`      | `(id: CalendarIdentifier) => Calendar`                        | —               | Custom calendar system      |
| `autoFocus`           | `boolean`                                                     | `false`         | Auto-focus on mount         |

#### 6.1.3 Calendar Events

| Event           | Payload        | Description          |
| --------------- | -------------- | -------------------- |
| `onChange`      | `DateValue`    | Date selected        |
| `onFocusChange` | `CalendarDate` | Focused date changes |

#### 6.1.4 CalendarGrid Props

| Prop           | Type                            | Default    | Description             |
| -------------- | ------------------------------- | ---------- | ----------------------- |
| `offset`       | `DateDuration`                  | —          | For multi-month display |
| `weekdayStyle` | `'narrow' \| 'short' \| 'long'` | `'narrow'` | Weekday name format     |

#### 6.1.5 CalendarCell Render Props

`{ isSelected, isDisabled, isUnavailable, isInvalid, isPressed, isFocused, isFocusVisible, isOutsideMonth, formattedDate }`

---

### 6.2 RangeCalendar

**React Aria name:** `RangeCalendar`
**Category:** Date-Time

Same sub-components as Calendar, plus range-specific behavior.

#### 6.2.1 Key Differences from Calendar

| Prop                        | Type                           | Default    | Description                         |
| --------------------------- | ------------------------------ | ---------- | ----------------------------------- |
| `value`                     | `RangeValue<T>`                | —          | Range selection (start/end pair)    |
| `defaultValue`              | `RangeValue<T>`                | —          | Initial range                       |
| `allowsNonContiguousRanges` | `boolean`                      | —          | Allow ranges with unavailable dates |
| `visibleMonths`             | `number`                       | `1`        | Months displayed                    |
| `selectionAlignment`        | `'start' \| 'end' \| 'center'` | `'center'` | Visible month alignment             |

#### 6.2.2 RangeCalendar Events

| Event           | Payload                          | Description          |
| --------------- | -------------------------------- | -------------------- |
| `onChange`      | `RangeValue<MappedDateValue<T>>` | Range changes        |
| `onFocusChange` | `CalendarDate`                   | Focused date changes |

#### 6.2.3 CalendarCell Render Props (additional)

`{ isSelectionStart, isSelectionEnd }` — in addition to Calendar cell props.

---

### 6.3 DateField

**React Aria name:** `DateField`
**Category:** Date-Time

#### 6.3.1 Sub-components

- `DateField` — Root container.
- `Label` — Field label.
- `DateInput` — Container for segments.
- `DateSegment` — Individual editable part (month/day/year/etc.).
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 6.3.2 DateField Props

| Prop                      | Type                                         | Default    | Description                         |
| ------------------------- | -------------------------------------------- | ---------- | ----------------------------------- |
| `value`                   | `DateValue \| null`                          | —          | Controlled date value               |
| `defaultValue`            | `DateValue \| null`                          | —          | Initial value                       |
| `minValue`                | `DateValue`                                  | —          | Minimum date                        |
| `maxValue`                | `DateValue`                                  | —          | Maximum date                        |
| `isDateUnavailable`       | `(date: DateValue) => boolean`               | —          | Date exclusion                      |
| `granularity`             | `Granularity`                                | —          | Precision level (day/minute/second) |
| `hourCycle`               | `12 \| 24`                                   | —          | Time format                         |
| `hideTimeZone`            | `boolean`                                    | `false`    | Hide timezone                       |
| `shouldForceLeadingZeros` | `boolean`                                    | —          | Pad numbers                         |
| `placeholderValue`        | `DateValue`                                  | —          | Placeholder format template         |
| `isDisabled`              | `boolean`                                    | —          | Disabled state                      |
| `isReadOnly`              | `boolean`                                    | —          | Read-only state                     |
| `isRequired`              | `boolean`                                    | —          | Required field                      |
| `isInvalid`               | `boolean`                                    | —          | Invalid state                       |
| `validate`                | `(value) => ValidationError \| true \| null` | —          | Custom validation                   |
| `validationBehavior`      | `'native' \| 'aria'`                         | `'native'` | Validation approach                 |
| `name`                    | `string`                                     | —          | Form name (ISO 8601)                |
| `form`                    | `string`                                     | —          | Associated form ID                  |
| `autoComplete`            | `string`                                     | —          | Auto-fill hint                      |
| `autoFocus`               | `boolean`                                    | —          | Focus on mount                      |

#### 6.3.3 DateField Events

| Event           | Payload                      | Description    |
| --------------- | ---------------------------- | -------------- |
| `onChange`      | `MappedDateValue<T> \| null` | Value changes  |
| `onFocus`       | `FocusEvent`                 | Receives focus |
| `onBlur`        | `FocusEvent`                 | Loses focus    |
| `onFocusChange` | `boolean`                    | Focus changes  |
| `onKeyDown`     | `KeyboardEvent`              | Key pressed    |
| `onKeyUp`       | `KeyboardEvent`              | Key released   |

#### 6.3.4 DateInput Render Props

`{ isFocused, isDisabled, isInvalid }`

#### 6.3.5 DateSegment Render Props

`{ isFocused, isDisabled, isPlaceholder, isInvalid, text, type }`

**Segment types:** `'day' | 'month' | 'year' | 'hour' | 'minute' | 'second' | 'dayPeriod' | 'literal'`

---

### 6.4 DatePicker

**React Aria name:** `DatePicker`
**Category:** Date-Time

#### 6.4.1 Sub-components

- `DatePicker` — Root container.
- `Label` — Field label.
- `Group` — Input container.
- `DateInput` + `DateSegment` — Text-based date entry.
- `Button` — Calendar trigger.
- `Popover` — Calendar overlay.
- `Calendar` — Date selection grid.
- `Dialog` — Calendar dialog wrapper.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 6.4.2 DatePicker Props (in addition to DateField props)

| Prop                  | Type                         | Default     | Description              |
| --------------------- | ---------------------------- | ----------- | ------------------------ |
| `isOpen`              | `boolean`                    | —           | Controlled popover state |
| `defaultOpen`         | `boolean`                    | —           | Initial popover state    |
| `shouldCloseOnSelect` | `boolean \| (() => boolean)` | `true`      | Auto-close on select     |
| `firstDayOfWeek`      | `'sun' \| ... \| 'sat'`      | —           | Week start override      |
| `pageBehavior`        | `PageBehavior`               | `'visible'` | Calendar pagination      |

#### 6.4.3 DatePicker Events (in addition to DateField events)

| Event          | Payload   | Description          |
| -------------- | --------- | -------------------- |
| `onOpenChange` | `boolean` | Popover opens/closes |

---

### 6.5 DateRangePicker

**React Aria name:** `DateRangePicker`
**Category:** Date-Time

#### 6.5.1 Sub-components

Same as DatePicker but with two DateInput slots ("start" and "end") and RangeCalendar.

#### 6.5.2 Key Props (differences from DatePicker)

| Prop                        | Type                    | Default | Description                      |
| --------------------------- | ----------------------- | ------- | -------------------------------- |
| `value`                     | `RangeValue<T> \| null` | —       | Range value                      |
| `defaultValue`              | `RangeValue<T> \| null` | —       | Initial range                    |
| `allowsNonContiguousRanges` | `boolean`               | —       | Allow unavailable dates in range |
| `startName`                 | `string`                | —       | Form name for start date         |
| `endName`                   | `string`                | —       | Form name for end date           |
| `maxVisibleMonths`          | `number`                | `1`     | Calendar months shown            |
| `shouldFlip`                | `boolean`               | `true`  | Flip orientation                 |

#### 6.5.3 Events

| Event      | Payload                                  | Description   |
| ---------- | ---------------------------------------- | ------------- |
| `onChange` | `RangeValue<MappedDateValue<T>> \| null` | Range changes |

---

### 6.6 TimeField

**React Aria name:** `TimeField`
**Category:** Date-Time

#### 6.6.1 Sub-components

- `TimeField` — Root container.
- `Label` — Field label.
- `DateInput` + `DateSegment` — Time segment inputs.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 6.6.2 TimeField Props

| Prop                                                  | Type                             | Default    | Description           |
| ----------------------------------------------------- | -------------------------------- | ---------- | --------------------- |
| `value`                                               | `TimeValue \| null`              | —          | Controlled time value |
| `defaultValue`                                        | `TimeValue \| null`              | —          | Initial value         |
| `minValue`                                            | `TimeValue`                      | —          | Minimum time          |
| `maxValue`                                            | `TimeValue`                      | —          | Maximum time          |
| `granularity`                                         | `'hour' \| 'minute' \| 'second'` | `'minute'` | Smallest unit         |
| `hourCycle`                                           | `12 \| 24`                       | —          | Time format           |
| `hideTimeZone`                                        | `boolean`                        | —          | Hide timezone         |
| `shouldForceLeadingZeros`                             | `boolean`                        | —          | Pad numbers           |
| `placeholderValue`                                    | `TimeValue`                      | —          | Placeholder format    |
| Same form/validation/accessibility props as DateField |                                  |            |                       |

#### 6.6.3 Events

Same as DateField events.

---

## 7. Data Display Components

### 7.1 Table

**React Aria name:** `Table`
**Category:** Data Display

#### 7.1.1 Sub-components

- `Table` — Root table container.
- `TableHeader` — Column definitions container.
- `Column` — Individual column with optional sorting.
- `ColumnResizer` — Column width resize handle.
- `TableBody` — Row container.
- `Row` — Individual table row.
- `Cell` — Individual table cell.
- `ResizableTableContainer` — Wrapper enabling column resizing.
- `TableLoadMoreItem` — Infinite scroll trigger.
- `Checkbox` — Selection checkbox.

#### 7.1.2 Table Props

| Prop                     | Type                         | Default            | Description           |
| ------------------------ | ---------------------------- | ------------------ | --------------------- |
| `selectionMode`          | `'single' \| 'multiple'`     | —                  | Row selection mode    |
| `selectedKeys`           | `Iterable<Key> \| 'all'`     | —                  | Controlled selection  |
| `defaultSelectedKeys`    | `Iterable<Key> \| 'all'`     | —                  | Initial selection     |
| `sortDescriptor`         | `SortDescriptor`             | —                  | Current sort state    |
| `disabledKeys`           | `Iterable<Key>`              | —                  | Disabled rows         |
| `disallowEmptySelection` | `boolean`                    | —                  | Prevent clearing      |
| `dragAndDropHooks`       | `DragAndDropHooks`           | —                  | DND support           |
| `escapeKeyBehavior`      | `'none' \| 'clearSelection'` | `'clearSelection'` | Escape handling       |
| `selectionBehavior`      | `SelectionBehavior`          | `'toggle'`         | Selection interaction |
| `disabledBehavior`       | `DisabledBehavior`           | `'all'`            | Disabled behavior     |

#### 7.1.3 Table Events

| Event               | Payload          | Description       |
| ------------------- | ---------------- | ----------------- |
| `onSelectionChange` | `Selection`      | Selection changes |
| `onSortChange`      | `SortDescriptor` | Sort changes      |
| `onRowAction`       | `Key`            | Row activated     |

#### 7.1.4 Column Props

| Prop            | Type                       | Default | Description         |
| --------------- | -------------------------- | ------- | ------------------- |
| `id`            | `Key`                      | —       | Unique identifier   |
| `allowsSorting` | `boolean`                  | —       | Enable sorting      |
| `isRowHeader`   | `boolean`                  | —       | Mark as row header  |
| `defaultWidth`  | `ColumnSize \| null`       | —       | Default width       |
| `width`         | `ColumnSize \| null`       | —       | Fixed width         |
| `minWidth`      | `ColumnStaticSize \| null` | —       | Minimum width       |
| `maxWidth`      | `ColumnStaticSize \| null` | —       | Maximum width       |
| `textValue`     | `string`                   | —       | Text representation |

#### 7.1.5 Column Render Props

`{ allowsSorting, sortDirection }`

#### 7.1.6 Row Props

| Prop         | Type          | Default | Description            |
| ------------ | ------------- | ------- | ---------------------- |
| `id`         | `Key`         | —       | Unique identifier      |
| `columns`    | `Iterable<T>` | —       | Column data            |
| `isDisabled` | `boolean`     | —       | Disabled row           |
| `textValue`  | `string`      | —       | Text for accessibility |
| `href`       | `string`      | —       | Link destination       |

#### 7.1.7 Row Events

Press, hover events.

#### 7.1.8 Cell Props

| Prop        | Type     | Default | Description            |
| ----------- | -------- | ------- | ---------------------- |
| `id`        | `Key`    | —       | Unique identifier      |
| `colSpan`   | `number` | —       | Column span            |
| `textValue` | `string` | —       | Text for accessibility |

#### 7.1.9 ResizableTableContainer Events

| Event           | Payload                | Description          |
| --------------- | ---------------------- | -------------------- |
| `onResize`      | `Map<Key, ColumnSize>` | Column widths change |
| `onResizeStart` | `Map<Key, ColumnSize>` | Resize begins        |
| `onResizeEnd`   | `Map<Key, ColumnSize>` | Resize ends          |

#### 7.1.10 ColumnResizer Render Props

`{ isResizing }`

---

### 7.2 GridList

**React Aria name:** `GridList`
**Category:** Data Display

#### 7.2.1 Sub-components

- `GridList` — Container.
- `GridListItem` — Individual item.
- `GridListSection` — Item grouping.
- `GridListHeader` — Section header.
- `GridListLoadMoreItem` — Infinite scroll trigger.
- `Checkbox` — Selection indicator.

#### 7.2.2 GridList Props

| Prop                  | Type                     | Default   | Description          |
| --------------------- | ------------------------ | --------- | -------------------- |
| `layout`              | `'stack' \| 'grid'`      | `'stack'` | Item arrangement     |
| `selectionMode`       | `SelectionMode`          | —         | Selection behavior   |
| `selectedKeys`        | `Iterable<Key> \| 'all'` | —         | Controlled selection |
| `defaultSelectedKeys` | `Iterable<Key> \| 'all'` | —         | Initial selection    |
| `items`               | `Iterable<T>`            | —         | Data source          |
| `disabledKeys`        | `Iterable<Key>`          | —         | Disabled items       |
| `dragAndDropHooks`    | `DragAndDropHooks`       | —         | DND support          |
| `renderEmptyState`    | `() => ReactNode`        | —         | Empty state          |

#### 7.2.3 Events

| Event               | Payload     | Description       |
| ------------------- | ----------- | ----------------- |
| `onSelectionChange` | `Selection` | Selection changes |
| `onAction`          | `Key`       | Item activated    |

#### 7.2.4 GridListItem Props

| Prop         | Type      | Default | Description        |
| ------------ | --------- | ------- | ------------------ |
| `id`         | `Key`     | —       | Unique identifier  |
| `textValue`  | `string`  | —       | Accessibility text |
| `href`       | `string`  | —       | Link destination   |
| `isDisabled` | `boolean` | —       | Disabled state     |

#### 7.2.5 GridListItem Events

Press, hover events (onAction, onPress, onHoverStart, etc.).

#### 7.2.6 GridListItem Render Props

`{ selectionMode, selectionBehavior, allowsDragging, isSelected, isFocused, isFocusVisible, isHovered, isPressed, isDisabled }`

---

### 7.3 Meter

**React Aria name:** `Meter`
**Category:** Data Display

#### 7.3.1 Sub-components

- `Meter` — Root container.
- `Label` — Meter label.

#### 7.3.2 Props

| Prop            | Type                       | Default              | Description        |
| --------------- | -------------------------- | -------------------- | ------------------ |
| `value`         | `number`                   | `0`                  | Current value      |
| `minValue`      | `number`                   | `0`                  | Minimum value      |
| `maxValue`      | `number`                   | `100`                | Maximum value      |
| `valueLabel`    | `ReactNode`                | —                    | Custom value label |
| `formatOptions` | `Intl.NumberFormatOptions` | `{style: 'percent'}` | Number format      |

#### 7.3.3 Render Props

`{ percentage, valueText }`

---

### 7.4 ProgressBar

**React Aria name:** `ProgressBar`
**Category:** Data Display

#### 7.4.1 Sub-components

- `ProgressBar` — Root container.
- `Label` — Progress label.

#### 7.4.2 Props

| Prop              | Type                       | Default              | Description            |
| ----------------- | -------------------------- | -------------------- | ---------------------- |
| `value`           | `number`                   | `0`                  | Current progress       |
| `minValue`        | `number`                   | `0`                  | Minimum value          |
| `maxValue`        | `number`                   | `100`                | Maximum value          |
| `isIndeterminate` | `boolean`                  | —                    | Unknown progress state |
| `valueLabel`      | `ReactNode`                | —                    | Custom value label     |
| `formatOptions`   | `Intl.NumberFormatOptions` | `{style: 'percent'}` | Number format          |

#### 7.4.3 Render Props

`{ percentage, valueText, isIndeterminate }`

---

## 8. Layout & Utility Components

### 8.1 Separator

**React Aria name:** `Separator`
**Category:** Layout

#### 8.1.1 Props

| Prop          | Type                         | Default        | Description       |
| ------------- | ---------------------------- | -------------- | ----------------- |
| `orientation` | `'horizontal' \| 'vertical'` | `'horizontal'` | Layout direction  |
| `elementType` | `string`                     | —              | HTML element type |

---

### 8.2 Group

**React Aria name:** `Group`
**Category:** Layout

#### 8.2.1 Props

| Prop         | Type                                    | Default   | Description     |
| ------------ | --------------------------------------- | --------- | --------------- |
| `role`       | `'group' \| 'region' \| 'presentation'` | `'group'` | ARIA role       |
| `isDisabled` | `boolean`                               | —         | Disabled state  |
| `isInvalid`  | `boolean`                               | —         | Invalid state   |
| `isReadOnly` | `boolean`                               | —         | Read-only state |

#### 8.2.2 Events

| Event           | Payload      | Description   |
| --------------- | ------------ | ------------- |
| `onHoverStart`  | `HoverEvent` | Hover begins  |
| `onHoverEnd`    | `HoverEvent` | Hover ends    |
| `onHoverChange` | `boolean`    | Hover changes |

---

### 8.3 Toolbar

**React Aria name:** `Toolbar`
**Category:** Layout

#### 8.3.1 Props

| Prop          | Type          | Default        | Description      |
| ------------- | ------------- | -------------- | ---------------- |
| `orientation` | `Orientation` | `'horizontal'` | Layout direction |

**Manages arrow key navigation among child controls. Provides context to ToggleButtonGroup and Separator.**

---

### 8.4 Form

**React Aria name:** `Form`
**Category:** Utility

#### 8.4.1 Props

| Prop                 | Type                                                        | Default    | Description                      |
| -------------------- | ----------------------------------------------------------- | ---------- | -------------------------------- |
| `action`             | `string \| ((formData: FormData) => void \| Promise<void>)` | —          | Submission target                |
| `autoComplete`       | `'off' \| 'on'`                                             | —          | Autocomplete behavior            |
| `autoCapitalize`     | `string`                                                    | —          | Capitalization control           |
| `encType`            | `string`                                                    | —          | Data encoding                    |
| `method`             | `'get' \| 'post' \| 'dialog'`                               | —          | HTTP method                      |
| `target`             | `'_blank' \| '_self' \| '_parent' \| '_top'`                | —          | Response target                  |
| `role`               | `'search' \| 'presentation'`                                | —          | ARIA role                        |
| `validationBehavior` | `'native' \| 'aria'`                                        | `'native'` | Validation approach              |
| `validationErrors`   | `ValidationErrors`                                          | —          | Server-side errors by field name |

#### 8.4.2 Events

| Event       | Payload     | Description             |
| ----------- | ----------- | ----------------------- |
| `onSubmit`  | `FormEvent` | Form submitted          |
| `onReset`   | `FormEvent` | Form reset              |
| `onInvalid` | `FormEvent` | Invalid field on submit |

---

### 8.5 FieldError

**React Aria name:** `FieldError`
**Category:** Utility (used within form fields)

#### 8.5.1 Props

| Prop        | Type                                                | Default | Description           |
| ----------- | --------------------------------------------------- | ------- | --------------------- |
| `children`  | `ReactNode \| ((v: ValidationResult) => ReactNode)` | —       | Error message content |
| `className` | `string`                                            | —       | CSS class             |
| `style`     | `CSSProperties`                                     | —       | Inline styles         |

---

### 8.6 DropZone

**React Aria name:** `DropZone`
**Category:** Utility / Drag & Drop

#### 8.6.1 Props

| Prop               | Type                                                                      | Default | Description          |
| ------------------ | ------------------------------------------------------------------------- | ------- | -------------------- |
| `isDisabled`       | `boolean`                                                                 | —       | Disabled state       |
| `getDropOperation` | `(types: DragTypes, allowedOperations: DropOperation[]) => DropOperation` | —       | Drop operation logic |

#### 8.6.2 Events

| Event            | Payload             | Description                |
| ---------------- | ------------------- | -------------------------- |
| `onDrop`         | `DropEvent`         | Valid drag drops on target |
| `onDropActivate` | `DropActivateEvent` | Drag held over target      |
| `onDropEnter`    | `DropEnterEvent`    | Drag enters target         |
| `onDropExit`     | `DropExitEvent`     | Drag exits target          |
| `onDropMove`     | `DropMoveEvent`     | Drag moves within target   |
| `onHoverStart`   | `HoverEvent`        | Hover begins               |
| `onHoverEnd`     | `HoverEvent`        | Hover ends                 |
| `onHoverChange`  | `boolean`           | Hover changes              |

#### 8.6.3 Render Props

`{ isFocusVisible, isDropTarget }`

---

### 8.7 FocusScope

**React Aria name:** `FocusScope`
**Category:** Utility

#### 8.7.1 Props

| Prop           | Type        | Default | Description                                  |
| -------------- | ----------- | ------- | -------------------------------------------- |
| `autoFocus`    | `boolean`   | —       | Auto-focus first focusable element on mount  |
| `contain`      | `boolean`   | —       | Trap focus inside scope                      |
| `restoreFocus` | `boolean`   | —       | Restore focus to previous element on unmount |
| `children`     | `ReactNode` | —       | Scope contents                               |

#### 8.7.2 Hook: `useFocusManager`

Returns `FocusManager` with methods:

- `focusNext({ wrap?: boolean })` — Move focus forward
- `focusPrevious({ wrap?: boolean })` — Move focus backward

---

### 8.8 VisuallyHidden

**React Aria name:** `VisuallyHidden`
**Category:** Utility
**Hook:** `useVisuallyHidden`

#### 8.8.1 Props

| Prop          | Type                              | Default | Description                                 |
| ------------- | --------------------------------- | ------- | ------------------------------------------- |
| `children`    | `ReactNode`                       | —       | Content to hide visually                    |
| `elementType` | `string \| JSXElementConstructor` | `'div'` | Container element                           |
| `isFocusable` | `boolean`                         | —       | Becomes visible on focus (e.g., skip links) |

---

## 9. Color Components

### 9.1 ColorPicker

**React Aria name:** `ColorPicker`
**Category:** Specialized / Color

#### 9.1.1 Sub-components

- `ColorPicker` — Root state management wrapper.
- Composes: `ColorArea`, `ColorSlider`, `ColorField`, `ColorSwatch`.

#### 9.1.2 Props

| Prop           | Type              | Default | Description      |
| -------------- | ----------------- | ------- | ---------------- |
| `value`        | `string \| Color` | —       | Controlled color |
| `defaultValue` | `string \| Color` | —       | Initial color    |

#### 9.1.3 Events

| Event      | Payload | Description   |
| ---------- | ------- | ------------- |
| `onChange` | `Color` | Color changes |

---

### 9.2 ColorArea

**React Aria name:** `ColorArea`
**Category:** Specialized / Color

#### 9.2.1 Sub-components

- `ColorArea` — Two-dimensional gradient control.
- `ColorThumb` — Draggable thumb indicator.

#### 9.2.2 Props

| Prop           | Type              | Default | Description                       |
| -------------- | ----------------- | ------- | --------------------------------- |
| `value`        | `string \| Color` | —       | Controlled color                  |
| `defaultValue` | `string \| Color` | —       | Initial color                     |
| `xChannel`     | `ColorChannel`    | —       | Horizontal axis channel           |
| `yChannel`     | `ColorChannel`    | —       | Vertical axis channel             |
| `colorSpace`   | `ColorSpace`      | —       | Color space ('rgb', 'hsl', 'hsb') |
| `isDisabled`   | `boolean`         | —       | Disabled state                    |
| `xName`        | `string`          | —       | Form name for x channel           |
| `yName`        | `string`          | —       | Form name for y channel           |
| `form`         | `string`          | —       | Associated form ID                |

#### 9.2.3 Events

| Event         | Payload | Description               |
| ------------- | ------- | ------------------------- |
| `onChange`    | `Color` | Color changes during drag |
| `onChangeEnd` | `Color` | Drag completes            |

---

### 9.3 ColorSlider

**React Aria name:** `ColorSlider`
**Category:** Specialized / Color

#### 9.3.1 Sub-components

- `ColorSlider` — Root container.
- `Label` — Slider label.
- `SliderOutput` — Value display.
- `SliderTrack` — Visual track.
- `ColorThumb` — Draggable thumb.

#### 9.3.2 Props

| Prop           | Type                         | Default        | Description       |
| -------------- | ---------------------------- | -------------- | ----------------- |
| `channel`      | `ColorChannel`               | **required**   | Channel to adjust |
| `value`        | `string \| Color`            | —              | Controlled color  |
| `defaultValue` | `string \| Color`            | —              | Initial color     |
| `colorSpace`   | `ColorSpace`                 | —              | Color space       |
| `isDisabled`   | `boolean`                    | —              | Disabled state    |
| `orientation`  | `'horizontal' \| 'vertical'` | `'horizontal'` | Layout            |
| `name`         | `string`                     | —              | Form name         |

#### 9.3.3 Events

| Event         | Payload | Description               |
| ------------- | ------- | ------------------------- |
| `onChange`    | `Color` | Color changes during drag |
| `onChangeEnd` | `Color` | Drag completes            |

---

### 9.4 ColorWheel

**React Aria name:** `ColorWheel`
**Category:** Specialized / Color

#### 9.4.1 Sub-components

- `ColorWheel` — Root circular component.
- `ColorWheelTrack` — Circular gradient track.
- `ColorThumb` — Draggable thumb.

#### 9.4.2 Props

| Prop           | Type              | Default               | Description            |
| -------------- | ----------------- | --------------------- | ---------------------- |
| `value`        | `string \| Color` | `'hsl(0, 100%, 50%)'` | Controlled color       |
| `defaultValue` | `string \| Color` | —                     | Initial color          |
| `outerRadius`  | `number`          | **required**          | Outer circle dimension |
| `innerRadius`  | `number`          | **required**          | Inner circle dimension |
| `isDisabled`   | `boolean`         | —                     | Disabled state         |
| `name`         | `string`          | —                     | Form name              |
| `form`         | `string`          | —                     | Associated form ID     |

#### 9.4.3 Events

| Event         | Payload | Description               |
| ------------- | ------- | ------------------------- |
| `onChange`    | `Color` | Color changes during drag |
| `onChangeEnd` | `Color` | Drag completes            |

---

### 9.5 ColorField

**React Aria name:** `ColorField`
**Category:** Specialized / Color

#### 9.5.1 Sub-components

- `ColorField` — Root container.
- `Label` — Field label.
- `Input` — Text input.
- `Text` (slot="description") — Helper text.
- `FieldError` — Error message.

#### 9.5.2 Props

| Prop                 | Type                                         | Default    | Description              |
| -------------------- | -------------------------------------------- | ---------- | ------------------------ |
| `value`              | `string \| Color \| null`                    | —          | Controlled color         |
| `defaultValue`       | `string \| Color \| null`                    | —          | Initial color            |
| `channel`            | `ColorChannel`                               | —          | Specific channel to edit |
| `colorSpace`         | `ColorSpace`                                 | —          | Color space context      |
| `isDisabled`         | `boolean`                                    | —          | Disabled state           |
| `isReadOnly`         | `boolean`                                    | —          | Read-only state          |
| `isRequired`         | `boolean`                                    | —          | Required field           |
| `isInvalid`          | `boolean`                                    | —          | Invalid state            |
| `isWheelDisabled`    | `boolean`                                    | —          | Disable scroll changes   |
| `placeholder`        | `string`                                     | —          | Placeholder text         |
| `name`               | `string`                                     | —          | Form name                |
| `validate`           | `(value) => ValidationError \| true \| null` | —          | Custom validation        |
| `validationBehavior` | `'native' \| 'aria'`                         | `'native'` | Validation approach      |

#### 9.5.3 Events

| Event           | Payload         | Description    |
| --------------- | --------------- | -------------- |
| `onChange`      | `Color \| null` | Color changes  |
| `onFocus`       | `FocusEvent`    | Receives focus |
| `onBlur`        | `FocusEvent`    | Loses focus    |
| `onFocusChange` | `boolean`       | Focus changes  |

---

### 9.6 ColorSwatch

**React Aria name:** `ColorSwatch`
**Category:** Specialized / Color

#### 9.6.1 Props

| Prop        | Type              | Default | Description              |
| ----------- | ----------------- | ------- | ------------------------ |
| `color`     | `string \| Color` | —       | Color value to display   |
| `colorName` | `string`          | —       | Accessible name override |

---

### 9.7 ColorSwatchPicker

**React Aria name:** `ColorSwatchPicker`
**Category:** Specialized / Color

#### 9.7.1 Sub-components

- `ColorSwatchPicker` — Container.
- `ColorSwatchPickerItem` — Individual swatch option.

#### 9.7.2 Props

| Prop           | Type                | Default  | Description                  |
| -------------- | ------------------- | -------- | ---------------------------- |
| `value`        | `string \| Color`   | —        | Selected color (controlled)  |
| `defaultValue` | `string \| Color`   | —        | Initial color (uncontrolled) |
| `layout`       | `'grid' \| 'stack'` | `'grid'` | Arrangement                  |

#### 9.7.3 Events

| Event      | Payload | Description       |
| ---------- | ------- | ----------------- |
| `onChange` | `Color` | Selection changes |

#### 9.7.4 ColorSwatchPickerItem Props

| Prop         | Type              | Default      | Description    |
| ------------ | ----------------- | ------------ | -------------- |
| `color`      | `string \| Color` | **required** | Swatch color   |
| `isDisabled` | `boolean`         | —            | Disabled state |

---

### 9.8 Color Type (shared across color components)

**`parseColor(value: string): Color`** — Parses color strings into Color objects.

**Color Object Methods:**

| Method                                | Returns                                      | Description          |
| ------------------------------------- | -------------------------------------------- | -------------------- |
| `toString(format?)`                   | `string`                                     | Serialize to string  |
| `toFormat(format)`                    | `Color`                                      | Convert color space  |
| `clone()`                             | `Color`                                      | Duplicate            |
| `toHexInt()`                          | `number`                                     | Integer hex          |
| `getChannelValue(channel)`            | `number`                                     | Channel value        |
| `withChannelValue(channel, value)`    | `Color`                                      | Set channel          |
| `getChannelRange(channel)`            | `{ min, max, step }`                         | Channel bounds       |
| `getChannelName(channel, locale)`     | `string`                                     | Localized name       |
| `getChannelFormatOptions(channel)`    | `Intl.NumberFormatOptions`                   | Format options       |
| `formatChannelValue(channel, locale)` | `string`                                     | Formatted display    |
| `getColorSpace()`                     | `ColorSpace`                                 | Current space        |
| `getColorChannels()`                  | `[ColorChannel, ColorChannel, ColorChannel]` | Channel list         |
| `getColorName(locale)`                | `string`                                     | Localized color name |
| `getHueName(locale)`                  | `string`                                     | Localized hue name   |

---

## 10. Miscellaneous Components

### 10.1 Disclosure (Accordion item)

**React Aria name:** `Disclosure`
**Category:** Navigation (Accordion)

#### 10.1.1 Sub-components

- `Disclosure` — Root expandable component.
- `DisclosureHeader` — Heading containing the trigger button.
- `DisclosurePanel` — Collapsible content.

#### 10.1.2 Disclosure Props

| Prop              | Type      | Default | Description                   |
| ----------------- | --------- | ------- | ----------------------------- |
| `isExpanded`      | `boolean` | —       | Controlled expansion          |
| `defaultExpanded` | `boolean` | —       | Initial state                 |
| `isDisabled`      | `boolean` | —       | Disabled state                |
| `id`              | `Key`     | —       | ID for use in DisclosureGroup |

#### 10.1.3 Events

| Event              | Payload   | Description       |
| ------------------ | --------- | ----------------- |
| `onExpandedChange` | `boolean` | Expansion changes |

#### 10.1.4 DisclosurePanel Props

| Prop   | Type                  | Default   | Description        |
| ------ | --------------------- | --------- | ------------------ |
| `role` | `'group' \| 'region'` | `'group'` | Accessibility role |

#### 10.1.5 Render Props

`{ isExpanded, isDisabled }`

---

### 10.2 DisclosureGroup (Accordion)

**React Aria name:** `DisclosureGroup`
**Category:** Navigation (Accordion)

#### 10.2.1 Props

| Prop                     | Type            | Default | Description                   |
| ------------------------ | --------------- | ------- | ----------------------------- |
| `allowsMultipleExpanded` | `boolean`       | —       | Allow multiple expanded items |
| `expandedKeys`           | `Iterable<Key>` | —       | Controlled expanded items     |
| `defaultExpandedKeys`    | `Iterable<Key>` | —       | Initial expanded items        |
| `isDisabled`             | `boolean`       | —       | Disable all items             |

#### 10.2.2 Events

| Event              | Payload    | Description           |
| ------------------ | ---------- | --------------------- |
| `onExpandedChange` | `Set<Key>` | Items expand/collapse |

---

### 10.3 Tree

**React Aria name:** `Tree`
**Category:** Navigation / Data Display

#### 10.3.1 Sub-components

- `Tree` — Root container.
- `TreeItem` — Individual expandable/selectable node.
- `TreeItemContent` — Render prop wrapper for item UI.
- `TreeSection` — Group of related items.
- `TreeHeader` — Section label.
- `TreeLoadMoreItem` — Infinite scroll trigger.

#### 10.3.2 Tree Props

| Prop                  | Type                         | Default            | Description           |
| --------------------- | ---------------------------- | ------------------ | --------------------- |
| `items`               | `Iterable<T>`                | —                  | Data source           |
| `selectionMode`       | `'single' \| 'multiple'`     | —                  | Selection mode        |
| `selectedKeys`        | `Iterable<Key>`              | —                  | Controlled selection  |
| `defaultSelectedKeys` | `Iterable<Key>`              | —                  | Initial selection     |
| `expandedKeys`        | `Iterable<Key>`              | —                  | Controlled expansion  |
| `defaultExpandedKeys` | `Iterable<Key>`              | —                  | Initial expansion     |
| `disabledKeys`        | `Iterable<Key>`              | —                  | Disabled items        |
| `disabledBehavior`    | `'all' \| 'selection'`       | `'all'`            | Disabled behavior     |
| `dragAndDropHooks`    | `DragAndDropHooks`           | —                  | DND support           |
| `renderEmptyState`    | `() => ReactNode`            | —                  | Empty state           |
| `escapeKeyBehavior`   | `'clearSelection' \| 'none'` | `'clearSelection'` | Escape handling       |
| `selectionBehavior`   | `'toggle'`                   | —                  | Selection interaction |

#### 10.3.3 Tree Events

| Event               | Payload     | Description           |
| ------------------- | ----------- | --------------------- |
| `onSelectionChange` | `Selection` | Selection changes     |
| `onExpandedChange`  | `Set<Key>`  | Items expand/collapse |
| `onAction`          | `Key`       | Item activated        |

#### 10.3.4 TreeItem Props

| Prop            | Type        | Default      | Description                       |
| --------------- | ----------- | ------------ | --------------------------------- |
| `title`         | `ReactNode` | **required** | Display label                     |
| `id`            | `Key`       | —            | Unique identifier                 |
| `textValue`     | `string`    | —            | Typeahead text                    |
| `isDisabled`    | `boolean`   | —            | Disabled state                    |
| `hasChildItems` | `boolean`   | —            | Has children (even if not loaded) |
| `href`          | `string`    | —            | Link URL                          |

#### 10.3.5 TreeItem Events

Press, hover events (onAction, onPress, onHoverStart, etc.).

#### 10.3.6 TreeItemContent Render Props

`{ selectionMode, selectionBehavior, hasChildItems, isExpanded, isDisabled, allowsDragging }`

---

### 10.4 Autocomplete

**React Aria name:** `Autocomplete`
**Category:** Utility / Selection

#### 10.4.1 Props

| Prop                    | Type                                       | Default | Description                      |
| ----------------------- | ------------------------------------------ | ------- | -------------------------------- |
| `inputValue`            | `string`                                   | —       | Controlled search value          |
| `defaultInputValue`     | `string`                                   | —       | Initial search value             |
| `filter`                | `(textValue, inputValue, node) => boolean` | —       | Custom filter logic              |
| `disableAutoFocusFirst` | `boolean`                                  | `false` | Prevent auto-focus on first item |
| `disableVirtualFocus`   | `boolean`                                  | `false` | Disable virtual focus            |

#### 10.4.2 Events

| Event           | Payload  | Description   |
| --------------- | -------- | ------------- |
| `onInputChange` | `string` | Input changes |

**Wraps:** SearchField/TextField (input) + Menu/ListBox/TagGroup/GridList/Table (collection).

---

### 10.5 Virtualizer

**React Aria name:** `Virtualizer`
**Category:** Utility / Performance

#### 10.5.1 Props

| Prop            | Type                           | Default | Description          |
| --------------- | ------------------------------ | ------- | -------------------- |
| `layout`        | `LayoutClass<O> \| ILayout<O>` | —       | Layout algorithm     |
| `layoutOptions` | `O`                            | —       | Layout configuration |

**Layout types:** `ListLayout`, `GridLayout`, `WaterfallLayout`, `TableLayout`

---

## 11. Components NOT in Our Spec

The following React Aria components exist but are NOT in the ars-ui spec:

1. **Autocomplete** — Wraps SearchField + collection for filtered selection
2. **Disclosure** — Expandable section (our spec has accordion in navigation, which maps to DisclosureGroup)
3. **DisclosureGroup** — Accordion (maps to our accordion)
4. **GridList** — Interactive grid/list with selection, DND (we may want this)
5. **Tree** — Hierarchical tree view (our spec has tree-view)
6. **Virtualizer** — Virtual scrolling for large lists
7. **Toast** — Toast notification system (our spec likely has toast in overlay)
8. **ColorThumb** — Shared thumb component for color controls
9. **ColorWheelTrack** — Track for ColorWheel
10. **ToggleButtonGroup** — Grouped toggle buttons with managed selection
11. **SharedElementTransition** — View transition animations (new/experimental)
12. **Pressable** — Low-level pressable element (new, no docs page yet)
13. **SelectionIndicator** — Selection indicator component

### 11.1 React Aria hooks (standalone @react-aria packages)

These hooks provide lower-level building blocks:

| Hook Package                  | Description                                                                                                                        |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `@react-aria/button`          | useButton, useToggleButton                                                                                                         |
| `@react-aria/checkbox`        | useCheckbox, useCheckboxGroup                                                                                                      |
| `@react-aria/calendar`        | useCalendar, useRangeCalendar, useCalendarGrid, useCalendarCell                                                                    |
| `@react-aria/color`           | useColorArea, useColorField, useColorSlider, useColorWheel                                                                         |
| `@react-aria/combobox`        | useComboBox                                                                                                                        |
| `@react-aria/datepicker`      | useDateField, useDatePicker, useDateRangePicker, useTimeField                                                                      |
| `@react-aria/dialog`          | useDialog                                                                                                                          |
| `@react-aria/disclosure`      | useDisclosure, useDisclosureGroup                                                                                                  |
| `@react-aria/dnd`             | useDrag, useDrop, useDragAndDrop                                                                                                   |
| `@react-aria/focus`           | useFocusRing, FocusScope, useFocusManager                                                                                          |
| `@react-aria/form`            | useForm                                                                                                                            |
| `@react-aria/grid`            | useGrid, useGridCell, useGridRow                                                                                                   |
| `@react-aria/gridlist`        | useGridList, useGridListItem                                                                                                       |
| `@react-aria/i18n`            | useFilter, useCollator, useDateFormatter, useNumberFormatter, useLocale                                                            |
| `@react-aria/interactions`    | usePress, useHover, useFocus, useFocusVisible, useFocusWithin, useKeyboard, useMove, useLongPress                                  |
| `@react-aria/label`           | useLabel                                                                                                                           |
| `@react-aria/landmark`        | useLandmark                                                                                                                        |
| `@react-aria/link`            | useLink                                                                                                                            |
| `@react-aria/listbox`         | useListBox, useOption                                                                                                              |
| `@react-aria/live-announcer`  | announce, clearAnnouncer                                                                                                           |
| `@react-aria/menu`            | useMenu, useMenuItem, useMenuTrigger, useSubmenuTrigger                                                                            |
| `@react-aria/meter`           | useMeter                                                                                                                           |
| `@react-aria/numberfield`     | useNumberField                                                                                                                     |
| `@react-aria/overlays`        | useOverlayTrigger, useOverlayPosition, useModal, DismissButton                                                                     |
| `@react-aria/progress`        | useProgressBar                                                                                                                     |
| `@react-aria/radio`           | useRadioGroup, useRadio                                                                                                            |
| `@react-aria/searchfield`     | useSearchField                                                                                                                     |
| `@react-aria/select`          | useSelect                                                                                                                          |
| `@react-aria/selection`       | useSelectableCollection, useSelectableItem, useSelectableList                                                                      |
| `@react-aria/separator`       | useSeparator                                                                                                                       |
| `@react-aria/slider`          | useSlider, useSliderThumb                                                                                                          |
| `@react-aria/spinbutton`      | useSpinButton                                                                                                                      |
| `@react-aria/ssr`             | useSSRSafeId, useIsSSR                                                                                                             |
| `@react-aria/steplist`        | useStepList                                                                                                                        |
| `@react-aria/switch`          | useSwitch                                                                                                                          |
| `@react-aria/table`           | useTable, useTableCell, useTableRow, useTableHeaderRow, useTableColumnHeader, useTableSelectAllCheckbox, useTableSelectionCheckbox |
| `@react-aria/tabs`            | useTabList, useTab, useTabPanel                                                                                                    |
| `@react-aria/tag`             | useTagGroup, useTag                                                                                                                |
| `@react-aria/textfield`       | useTextField                                                                                                                       |
| `@react-aria/toast`           | useToast, useToastRegion                                                                                                           |
| `@react-aria/toggle`          | useToggleButton                                                                                                                    |
| `@react-aria/toolbar`         | useToolbar                                                                                                                         |
| `@react-aria/tooltip`         | useTooltip, useTooltipTrigger                                                                                                      |
| `@react-aria/tree`            | useTree, useTreeItem                                                                                                               |
| `@react-aria/utils`           | Various utility hooks and functions                                                                                                |
| `@react-aria/virtualizer`     | Virtualizer, useVirtualizer                                                                                                        |
| `@react-aria/visually-hidden` | useVisuallyHidden, VisuallyHidden                                                                                                  |

---

## 12. Shared Patterns Across All Components

### 12.1 Common Props (present on most components)

- `className` — String or function receiving render props
- `style` — CSSProperties or function receiving render props
- `children` — ReactNode or function receiving render props
- `render` — Custom DOM render function for composition
- `slot` — Slot name for parent prop inheritance
- `id`, `dir`, `lang`, `hidden`, `inert`, `translate` — Standard HTML attributes
- `aria-label`, `aria-labelledby`, `aria-describedby`, `aria-details` — ARIA attributes

### 12.2 Common Event Handlers

All interactive components support standard React DOM events (mouse, pointer, touch, keyboard, animation, transition, scroll, wheel) plus their capture variants.

### 12.3 Render Props Pattern

Components accept function children that receive state objects for conditional rendering:

```tsx
<Component>
  {(renderProps) => (
    <div className={renderProps.isSelected ? "selected" : ""}>...</div>
  )}
</Component>
```

### 12.4 Collection Components Pattern

Components like ListBox, Menu, Select, ComboBox, TagGroup, GridList, Table, Tree use a shared collections API:

- `items` prop for dynamic data
- Function children for item rendering
- `disabledKeys` for disabling specific items
- Section components for grouping
- `LoadMoreItem` components for infinite scroll

### 12.5 Validation Pattern

Form-integrated components support:

- `validate` — Custom validation function
- `validationBehavior` — `'native'` (HTML) or `'aria'` (ARIA attributes)
- `isRequired`, `isInvalid` — State props
- `FieldError` sub-component for error display

### 12.6 Controlled/Uncontrolled Pattern

Most stateful components support both:

- Controlled: `value`/`selectedKey`/`isOpen` + `onChange`/`onSelectionChange`/`onOpenChange`
- Uncontrolled: `defaultValue`/`defaultSelectedKey`/`defaultOpen`
