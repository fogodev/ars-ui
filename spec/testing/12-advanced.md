# Advanced Testing

## 1. Specialized Component Testing

Components with browser API dependencies require specific testing strategies.

### 1.1 Timer/Clock-Dependent Components

Components like Toast, Carousel (auto-play), and Timer depend on `setTimeout`/`setInterval`. Tests should use controlled time advancement:

```rust
#[cfg(test)]
mod timer_tests {
    use std::{
        cell::RefCell,
        rc::Rc,
        time::Duration
    };

    /// A deterministic timer for testing time-dependent behavior.
    ///
    /// `advance(duration)` fires all callbacks with delay <= duration, then reduces
    /// remaining callbacks' delays by that amount. Note: `advance(Duration::ZERO)` fires all
    /// zero-delay callbacks (those scheduled with `setTimeout(f, 0)`).
    struct FakeTimer {
        callbacks: Rc<RefCell<Vec<(u64, Box<dyn FnOnce()>)>>>,
        frame_callbacks: Rc<RefCell<Vec<Box<dyn FnOnce(f64)>>>>,
        elapsed_ms: Cell<u64>,
    }

    impl FakeTimer {
        fn advance(&self, duration: Duration) {
            let ms = duration.as_millis() as u64;
            let mut cbs = self.callbacks.borrow_mut();
            let (ready, remaining): (Vec<_>, Vec<_>) =
                cbs.drain(..).partition(|(delay, _)| *delay <= ms);
            *cbs = remaining.into_iter()
                .map(|(d, cb)| (d.saturating_sub(ms), cb))
                .collect();
            drop(cbs);
            self.elapsed_ms.set(self.elapsed_ms.get() + ms);
            for (_, cb) in ready { cb(); }
        }

        /// Advance one animation frame, executing all requestAnimationFrame callbacks.
        fn advance_frame(&self) {
            for cb in self.frame_callbacks.borrow_mut().drain(..) {
                cb(self.elapsed_ms.get() as f64);
            }
        }
    }
}
```

#### 1.1.1 FakeTimer Integration with Effect System

To intercept browser timer APIs in `wasm_bindgen_test`, `FakeTimer` patches the global `setTimeout` and `setInterval` via JS interop:

```rust
#[wasm_bindgen(inline_js = "
    export function install_fake_timers(fake) {
        window.__real_setTimeout = window.setTimeout;
        window.__real_setInterval = window.setInterval;
        window.__real_rAF = window.requestAnimationFrame;
        window.setTimeout = (cb, ms) => fake.schedule(cb, ms);
        window.setInterval = (cb, ms) => fake.schedule_interval(cb, ms);
        window.requestAnimationFrame = (cb) => fake.schedule_frame(cb);
    }
    export function restore_real_timers() {
        window.setTimeout = window.__real_setTimeout;
        window.setInterval = window.__real_setInterval;
        window.requestAnimationFrame = window.__real_rAF;
    }
")]
extern "C" {
    fn install_fake_timers(fake: &FakeTimer);
    fn restore_real_timers();
}

impl Drop for FakeTimer {
    fn drop(&mut self) {
        restore_real_timers();
    }
}
```

`PendingEffect` setup closures that call `setTimeout` will automatically route through `FakeTimer` when installed. The test controls time advancement via `fake_timer.advance(Duration::from_millis(ms))`.

### 1.2 Canvas-Based Components (SignaturePad, ImageCropper, QRCode)

Canvas output cannot be tested via DOM assertions. **Separate stroke model tests
(unit, no canvas) from canvas output tests** to keep the fast-feedback loop tight.

#### 1.2.1 Stroke Model Tests (Unit, No Canvas)

Test the `SignatureData`, `SignatureStroke`, and `SignaturePoint` data model
independently of rendering. These are pure Rust tests that run without `wasm-pack`:

```rust
#[test]
fn signature_stroke_model_multi_point() {
    let mut data = SignatureData::default();
    assert!(data.is_empty());

    // Simulate a multi-point stroke
    let stroke = SignatureStroke {
        points: vec![
            SignaturePoint { x: 10.0, y: 20.0, pressure: 0.5, timestamp: 0.0 },
            SignaturePoint { x: 30.0, y: 40.0, pressure: 0.7, timestamp: 16.0 },
            SignaturePoint { x: 50.0, y: 30.0, pressure: 0.6, timestamp: 32.0 },
            SignaturePoint { x: 70.0, y: 50.0, pressure: 0.8, timestamp: 48.0 },
        ],
    };
    data.strokes.push(stroke);

    assert_eq!(data.point_count(), 4);
    assert!(!data.is_empty());

    let svg = data.to_svg_path();
    assert!(svg.starts_with("M10.0,20.0"));
    assert!(svg.contains("L70.0,50.0"));
}
```

#### 1.2.2 Multi-Point Stroke Simulation API

Define a test helper for generating multi-point strokes from a sequence of
`(x, y)` coordinates with simulated pressure and timing:

```rust
/// Simulate a continuous stroke from a sequence of (x, y) points.
/// Pressure ramps linearly from 0.3 to 0.8. Timestamps increment by 16ms.
fn simulate_stroke(points: &[(f64, f64)]) -> SignatureStroke {
    let n = points.len() as f64;
    SignatureStroke {
        points: points.iter().enumerate().map(|(i, &(x, y))| {
            SignaturePoint {
                x, y,
                pressure: 0.3 + (0.5 * i as f64 / n.max(1.0)),
                timestamp: i as f64 * 16.0,
            }
        }).collect(),
    }
}
```

#### 1.2.3 Canvas Output Tests (WASM, Visual Comparison)

- **Pixel-level**: use `canvas.toDataURL()` and compare against golden images
  using SSIM (Structural Similarity Index) with a **95% threshold** to allow
  for antialiasing and sub-pixel rendering differences across browsers.
- **Interaction**: verify that pointer events produce correct state transitions
  and context mutations (stroke data, crop region coordinates).
- Canvas output tests run in `wasm-pack test --headless` and are slower —
  keep them in a separate test module from stroke model unit tests.

**CI requirements**: Canvas golden-image tests require `--headless=new` (not legacy
headless) for consistent rendering. The SSIM threshold of 95% may need adjustment per
CI GPU/CPU configuration.

> **SSIM implementation:** Use the [`image-compare`](https://crates.io/crates/image-compare) crate (v0.4+) for structural similarity comparison. The `image_compare::rgba_hybrid_compare` function provides SSIM with a configurable threshold. Alternatively, for simpler byte-level comparison, use `image::ImageBuffer::pixels()` with a manual RMSE calculation.
>
> **CI Note:** Canvas tests using `toDataURL()` or `getContext('2d')` require a headed browser
> or the `--disable-gpu` + `--use-gl=swiftshader` flags for Chrome headless. Some CI
> environments (e.g., GitHub Actions with `ubuntu-latest`) may need `xvfb-run` for canvas
> rendering.

#### 1.2.4 Canvas Accessibility Tests

Canvas-based components must have accessible roles and labels for assistive technology:

```rust
#[wasm_bindgen_test]
async fn signature_pad_canvas_has_accessible_role() {
    // Mount SignaturePad
    let canvas = query_selector("canvas");
    assert_eq!(canvas.attr("role"), Some("img"));
    assert!(canvas.attr("aria-label").is_some(), "Canvas must have aria-label");
}

#[wasm_bindgen_test]
async fn qr_code_canvas_has_alt_text() {
    // Mount QrCode with value
    let canvas = query_selector("canvas");
    assert_eq!(canvas.attr("role"), Some("img"));
    // aria-label should contain the QR code value or a description
    assert!(canvas.attr("aria-label").expect("canvas must have aria-label").contains("QR code"));
}
```

**State-machine-level canvas accessibility tests** (no DOM required):

```rust
#[test]
fn signature_pad_canvas_has_accessible_label() {
    let props = signature_pad::Props::new("sp1").label("Your signature");
    let svc = Service::new(props);
    let api = svc.connect(&|_| {});
    let canvas_attrs = api.part_attrs(Part::Canvas);
    assert_role(&canvas_attrs, "img");
    assert_aria_label(&canvas_attrs, "Your signature");
}

#[test]
fn qr_code_canvas_has_alt_text() {
    let props = qr_code::Props::new("qr1").value("https://example.com").alt("QR code for example.com");
    let svc = Service::new(props);
    let api = svc.connect(&|_| {});
    let canvas_attrs = api.part_attrs(Part::Canvas);
    assert_role(&canvas_attrs, "img");
    assert_aria_label(&canvas_attrs, "QR code for example.com");
}
```

### 1.3 File API Components (FileUpload, DropZone)

Mock the browser File API:

```rust
// Create mock File objects via wasm-bindgen
let file = web_sys::File::new_with_str_sequence(
    &js_sys::Array::of1(&"content".into()),
    "test.txt",
).expect("File creation in test");

svc.send(file_upload::Event::FilesSelected(vec![file_upload::Item::from(file)]));
```

### 1.4 Clipboard Components

Mock `navigator.clipboard`:

```rust
#[wasm_bindgen(inline_js = "
    let _clipboard_text = '';
    export function install_clipboard_mock() {
        const mock = {
            writeText: async (text) => { _clipboard_text = text; },
            readText: async () => _clipboard_text,
        };
        Object.defineProperty(navigator, 'clipboard', {
            value: mock,
            writable: true,
            configurable: true,
        });
    }
    export function get_clipboard_text() { return _clipboard_text; }
    export function set_clipboard_text(text) { _clipboard_text = text; }
")]
extern "C" {
    fn install_clipboard_mock();
    fn get_clipboard_text() -> String;
    fn set_clipboard_text(text: &str);
}

/// Test helper: installs a mock clipboard API that works without HTTPS or user gesture.
/// Call at the start of clipboard tests. The mock is global — clipboard tests must run serially.
struct MockClipboard;
impl MockClipboard {
    fn install() -> Self {
        install_clipboard_mock();
        Self
    }
    fn read(&self) -> String { get_clipboard_text() }
    fn write(&self, text: &str) { set_clipboard_text(text); }
}
```

**CI requirements**: Clipboard tests require HTTPS or localhost context. In CI, use
`--headless=new` Chrome flag. Mock `navigator.clipboard` via JS interop for permission
scenarios.

---

## 2. Visual Regression Testing

While ars-ui is headless (no built-in styles), changes to data attributes and anatomy can break consumer CSS. Visual regression tests detect breaking attribute changes.

### 2.1 Strategy

1. Maintain a reference "styled storybook" with minimal CSS targeting `data-ars-*` attributes
2. Use Playwright screenshot comparison against the storybook as a CI step
3. Snapshot test all `data-ars-state` values as a breaking-change detection mechanism

### 2.2 AttrMap key stability tests

```rust
#[test]
fn data_state_values_are_stable() {
    // Verify all data-ars-state values are kebab-case and haven't changed
    let states = vec![
        (toggle::State::Off, "off"),
        (toggle::State::On, "on"),
        (dialog::State::Closed, "closed"),
        (dialog::State::Open, "open"),
        // ... all component states
    ];
    for (state, expected) in states {
        assert_eq!(state.to_string(), expected, "data-ars-state value changed");
    }
}
```

### 2.3 Playwright integration

```typescript
// tests/visual/dialog.spec.ts
import { test, expect } from "@playwright/test";

test("dialog open state", async ({ page }) => {
  await page.goto("/storybook/dialog");
  await page.click('[data-ars-part="trigger"]');
  await expect(page.locator('[data-ars-part="content"]')).toBeVisible();
  await expect(page).toHaveScreenshot("dialog-open.png");
});
```

> **CI integration:** The Playwright visual regression pipeline requires:
>
> 1. **Build:** `wasm-pack build --target web crates/ars-storybook` to produce the WASM storybook
> 2. **Serve:** `npx serve dist/` (or equivalent static server) on `localhost:8080`
> 3. **Test:** `npx playwright test --project=chromium` against the served storybook
> 4. **Artifacts:** Screenshot diffs are uploaded as CI artifacts for visual review
>
> This pipeline runs as a separate GitHub Actions job with `playwright` pre-installed on the runner.

---

## 3. Effect Cleanup Leak Detection

### 3.1 Systematic leak detection

```rust
#[cfg(test)]
mod leak_tests {
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn dialog_open_close_100x_no_leak() {
        let setup_count = Rc::new(Cell::new(0u32));
        let cleanup_count = Rc::new(Cell::new(0u32));

        let props = dialog::Props { id: "leak-test".into(), ..Default::default() };
        let mut svc = Service::<dialog::Machine>::new(props);
        let mut active_cleanups: Vec<Box<dyn FnOnce()>> = Vec::new();

        // PendingEffect::run() takes Rc<dyn Fn(M::Event)> on wasm32,
        // Arc<dyn Fn(M::Event) + Send + Sync> on native.
        #[cfg(target_arch = "wasm32")]
        let send_fn: Rc<dyn Fn(dialog::Event)> = Rc::new(|_| {});
        #[cfg(not(target_arch = "wasm32"))]
        let send_fn: Arc<dyn Fn(dialog::Event) + Send + Sync> = Arc::new(|_| {});

        for _ in 0..100 {
            // Open
            let result = svc.send(dialog::Event::Open);
            for effect in result.pending_effects {
                setup_count.set(setup_count.get() + 1);
                let cc = cleanup_count.clone();
                let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
                active_cleanups.push(Box::new(move || {
                    cc.set(cc.get() + 1);
                    cleanup();
                }));
            }

            // Close — run all cleanups, then process close effects
            let close_result = svc.send(dialog::Event::Close);
            for cleanup in active_cleanups.drain(..) {
                cleanup();
            }
            for effect in close_result.pending_effects {
                let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
                active_cleanups.push(Box::new(move || {
                    cleanup();
                }));
            }
        }

        assert_eq!(setup_count.get(), cleanup_count.get(),
            "effect setup/cleanup count mismatch: {} setups, {} cleanups",
            setup_count.get(), cleanup_count.get());
        assert!(active_cleanups.is_empty(), "dangling cleanups after final close");
    }

    #[test]
    fn rc_strong_count_no_cycles() {
        let props = dialog::Props { id: "rc-test".into(), ..Default::default() };
        let mut svc = Service::<dialog::Machine>::new(props);

        // PendingEffect::run() takes Rc<dyn Fn(M::Event)> on wasm32,
        // Arc<dyn Fn(M::Event) + Send + Sync> on native.
        #[cfg(target_arch = "wasm32")]
        let send_fn: Rc<dyn Fn(dialog::Event)> = Rc::new(|_| {});
        #[cfg(not(target_arch = "wasm32"))]
        let send_fn: Arc<dyn Fn(dialog::Event) + Send + Sync> = Arc::new(|_| {});

        #[cfg(target_arch = "wasm32")]
        let initial_count = Rc::strong_count(&send_fn);
        #[cfg(not(target_arch = "wasm32"))]
        let initial_count = Arc::strong_count(&send_fn);

        let result = svc.send(dialog::Event::Open);
        let mut cleanups = Vec::new();
        for effect in result.pending_effects {
            let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
            cleanups.push(cleanup);
        }

        // Cleanups may hold clones of send_fn
        #[cfg(target_arch = "wasm32")]
        let during_count = Rc::strong_count(&send_fn);
        #[cfg(not(target_arch = "wasm32"))]
        let during_count = Arc::strong_count(&send_fn);

        // Run cleanups
        for cleanup in cleanups {
            cleanup();
        }

        #[cfg(target_arch = "wasm32")]
        let after_count = Rc::strong_count(&send_fn);
        #[cfg(not(target_arch = "wasm32"))]
        let after_count = Arc::strong_count(&send_fn);
        assert_eq!(after_count, initial_count,
            "send_fn strong count leaked: initial={}, after cleanup={}",
            initial_count, after_count);
    }
}
```

### 3.2 Memory Safety and Leak Testing

Memory leak testing MUST be included in the CI checklist for every component that uses effects, signals, or closures. The following patterns MUST be tested:

**Signal Capture Leak Detection**: Verify that closures passed to effects do not hold strong references that create cycles:

```rust
#[test]
fn no_signal_capture_leak() {
    /// Mounts a component backed by a Service into a test DOM.
    /// Uses Rc<RefCell<Service>> to allow both rendering (immutable borrow)
    /// and event handling (mutable borrow).
    fn mount_component<M: Machine>(svc: Rc<RefCell<Service<M>>>) -> TestHandle {
        // ... adapter-specific mounting logic
        todo!("adapter-specific mounting")
    }

    let (service, weak_service) = {
        let svc = Rc::new(RefCell::new(Service::new(props)));
        let weak = Rc::downgrade(&svc);
        // Mount component, creating effects that capture `svc`
        let _handle = mount_component(Rc::clone(&svc));
        (svc, weak)
    };
    // Unmount component — should drop all effect closures
    drop(service);
    // If weak can still upgrade, something leaked
    assert!(weak_service.upgrade().is_none(), "Service leaked after unmount");
}
```

**Effect Cycle Detection**: Verify that `PendingEffect` setup/cleanup pairs are balanced.
There is no `Service::new_with_effect_tracking()` — track effects manually by wrapping
`effect.run()` calls:

```rust
#[test]
fn effects_balanced_on_lifecycle() {
    let setup_count = Rc::new(Cell::new(0u32));
    let cleanup_count = Rc::new(Cell::new(0u32));

    let props = dialog::Props { id: "balance-test".into(), ..Default::default() };
    let mut svc = Service::<dialog::Machine>::new(props);
    // PendingEffect::run() takes Rc<dyn Fn(M::Event)> on wasm32,
    // Arc<dyn Fn(M::Event) + Send + Sync> on native.
    #[cfg(target_arch = "wasm32")]
    let send_fn: Rc<dyn Fn(dialog::Event)> = Rc::new(|_| {});
    #[cfg(not(target_arch = "wasm32"))]
    let send_fn: Arc<dyn Fn(dialog::Event) + Send + Sync> = Arc::new(|_| {});
    let mut active_cleanups: Vec<CleanupFn> = Vec::new();

    // Track effects manually by wrapping effect.run() calls
    let mut run_effects = |result: &SendResult<dialog::Machine>,
                           svc: &Service<dialog::Machine>,
                           setups: &Rc<Cell<u32>>,
                           cleanups: &Rc<Cell<u32>>,
                           active: &mut Vec<CleanupFn>| {
        for effect in &result.pending_effects {
            setups.set(setups.get() + 1);
            let cc = cleanups.clone();
            let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
            active.push(Box::new(move || {
                cc.set(cc.get() + 1);
                cleanup();
            }));
        }
    };

    // Run through lifecycle: mount -> interact -> unmount
    let result = svc.send(dialog::Event::Open);
    run_effects(&result, &svc, &setup_count, &cleanup_count, &mut active_cleanups);

    let result = svc.send(dialog::Event::Close);
    for cleanup in active_cleanups.drain(..) { cleanup(); }
    run_effects(&result, &svc, &setup_count, &cleanup_count, &mut active_cleanups);

    let result = svc.send(dialog::Event::Open);
    run_effects(&result, &svc, &setup_count, &cleanup_count, &mut active_cleanups);

    // Collect all active cleanup functions and pass to unmount.
    // Service::unmount(active_cleanups) takes ownership of cleanups and runs them.
    let cleanups_for_unmount: Vec<CleanupFn> = active_cleanups.drain(..).collect();
    svc.unmount(cleanups_for_unmount);
    assert!(svc.is_unmounted());

    assert_eq!(setup_count.get(), cleanup_count.get(),
        "Setup/cleanup mismatch: {} setups vs {} cleanups",
        setup_count.get(), cleanup_count.get());
}
```

**Unmount Cleanup Verification**: Every component with `PendingEffect` entries MUST have
a test verifying that unmounting triggers all cleanup functions. Note that
`Service::unmount(active_cleanups: Vec<CleanupFn>)` takes the list of active cleanup
functions that the adapter has been tracking:

```rust
#[test]
fn dialog_cleanup_on_unmount() {
    let cleanup_ran = Rc::new(Cell::new(false));
    let props = dialog::Props { id: "unmount-test".into(), ..Default::default() };
    let mut svc = Service::<dialog::Machine>::new(props);
    // PendingEffect::run() takes Rc<dyn Fn(M::Event)> on wasm32,
    // Arc<dyn Fn(M::Event) + Send + Sync> on native.
    #[cfg(target_arch = "wasm32")]
    let send_fn: Rc<dyn Fn(dialog::Event)> = Rc::new(|_| {});
    #[cfg(not(target_arch = "wasm32"))]
    let send_fn: Arc<dyn Fn(dialog::Event) + Send + Sync> = Arc::new(|_| {});

    let result = svc.send(dialog::Event::Open);
    let mut active_cleanups: Vec<CleanupFn> = Vec::new();
    for effect in result.pending_effects {
        let cr = cleanup_ran.clone();
        let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
        active_cleanups.push(Box::new(move || {
            cr.set(true);
            cleanup();
        }));
    }

    assert!(!cleanup_ran.get());

    // Collect all active cleanup functions and pass to unmount
    let cleanups: Vec<CleanupFn> = active_cleanups.drain(..).collect();
    svc.unmount(cleanups);
    assert!(svc.is_unmounted());
    assert!(cleanup_ran.get(), "Dialog effects not cleaned up on unmount");
}
```

**CI Checklist Requirement**: The CI pipeline MUST include memory leak tests as a mandatory check. Components that fail leak tests MUST block the build. The following components are high-priority for leak testing due to their use of effects:

- Dialog (focus trap, inert, scroll lock, body style effects)
- Tooltip / Popover (positioning effects, ResizeObserver, event listeners)
- Combobox / Autocomplete (filter effects, collection observer)
- Toast (timer effects, auto-dismiss)
- Any component using `auto_update()` positioning

**Interleaved Effect Setup and Cancel**:

```rust
/// Tests that effects from send N are properly cancelled when send N+1
/// arrives before N's effect setup completes.
#[wasm_bindgen_test]
async fn interleaved_effect_setup_and_cancel() {
    let props = tooltip::Props { open_delay_ms: 500, ..Default::default() };
    let mut svc = Service::new(props);
    // PendingEffect::run() takes Rc<dyn Fn(M::Event)> on wasm32,
    // Arc<dyn Fn(M::Event) + Send + Sync> on native.
    #[cfg(target_arch = "wasm32")]
    let send_fn: Rc<dyn Fn(tooltip::Event)> = Rc::new(|_| {});
    #[cfg(not(target_arch = "wasm32"))]
    let send_fn: Arc<dyn Fn(tooltip::Event) + Send + Sync> = Arc::new(|_| {});

    // Send N: hover triggers delayed open effect
    let result1 = svc.send(tooltip::Event::PointerEnter);
    let effects1: Vec<_> = result1.pending_effects.into_iter().collect();
    assert!(!effects1.is_empty(), "hover must produce a delayed open effect");

    // Send N+1: pointer leaves before effect setup completes
    let result2 = svc.send(tooltip::Event::PointerLeave);
    // Effects from send N must appear in cancel_effects
    assert!(
        !result2.cancel_effects.is_empty(),
        "pointer leave must cancel the pending open effect from hover"
    );

    // Run the cancelled effect's setup — cleanup should be a no-op
    for effect in effects1 {
        let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
        cleanup(); // Should not panic or cause side effects
    }
}
```

### 3.3 Adapter-Side Leak Detection

Rust-side leak detection (Rc counts, setup/cleanup balancing) does not catch browser-side
leaks: dangling DOM event listeners, DOM nodes retained in closures, and orphaned
MutationObservers. Adapter tests MUST include browser-side cleanup verification.

**DOM listener tracking**: Before mounting, count active `addEventListener` calls on the
document and relevant DOM nodes. After unmounting, assert the count returns to baseline:

```rust
#[wasm_bindgen_test]
fn dialog_no_leaked_dom_listeners() {
    let tracker = DomListenerTracker::install(); // Patches addEventListener/removeEventListener
    let baseline = tracker.active_count();

    let harness = mount(Dialog::new().open(true));
    let during = tracker.active_count();
    assert!(during > baseline, "Dialog should add listeners when mounted");

    harness.unmount();
    let after = tracker.active_count();
    assert_eq!(after, baseline,
        "Leaked {} DOM listeners after unmount", after - baseline);
}
```

**`addEventListener` counting pattern**: The `DomListenerTracker` utility wraps
`EventTarget.addEventListener` and `EventTarget.removeEventListener` via JS interop to
maintain a global reference count. Tests use `active_count()` to detect leaks by comparing
listener counts before and after component lifecycle operations:

```rust
#[wasm_bindgen(inline_js = "
    let originalAdd = EventTarget.prototype.addEventListener;
    let originalRemove = EventTarget.prototype.removeEventListener;
    let listenerCount = 0;

    export function patchListeners() {
        EventTarget.prototype.addEventListener = function(...args) {
            listenerCount++;
            return originalAdd.apply(this, args);
        };
        EventTarget.prototype.removeEventListener = function(...args) {
            listenerCount--;
            return originalRemove.apply(this, args);
        };
    }

    export function unpatchListeners() {
        EventTarget.prototype.addEventListener = originalAdd;
        EventTarget.prototype.removeEventListener = originalRemove;
    }

    export function getListenerCount() { return listenerCount; }
")]
extern "C" {
    fn patchListeners();
    fn unpatchListeners();
    fn getListenerCount() -> i32;
}

struct DomListenerTracker;
impl DomListenerTracker {
    fn install() -> Self { patchListeners(); Self }
    fn active_count(&self) -> i32 { getListenerCount() }
}
impl Drop for DomListenerTracker {
    fn drop(&mut self) { unpatchListeners(); }
}
```

> **Test isolation:** `DomListenerTracker` patches `EventTarget.prototype.addEventListener` globally. In `wasm_bindgen_test` (which runs tests concurrently by default), multiple tests using this tracker will interfere. Tests using `DomListenerTracker` MUST be annotated with `#[serial]` (from the [`serial_test`](https://crates.io/crates/serial_test) crate) or run in single-threaded WASM mode.

**Dialog focus trap leak test**: Dialog's focus trap installs `keydown` and `focusin` listeners.
These MUST be removed on close/unmount:

```rust
#[wasm_bindgen_test]
fn dialog_focus_trap_listeners_cleaned_up() {
    let tracker = DomListenerTracker::install();
    let before = tracker.active_count();

    let harness = mount(Dialog::new().open(true));
    let during = tracker.active_count();
    assert!(during > before, "Focus trap should add listeners");

    harness.send(dialog::Event::Close);
    harness.fire_animation_end(); // Wait for exit animation
    harness.unmount();

    let after = tracker.active_count();
    assert_eq!(after, before, "All focus trap listeners should be cleaned up");
}
```

---

## 4. Touch Interaction Testing

> **CI note:** Touch event tests require headless Chrome with `--touch-events` flag. Not available in all CI environments. These tests should be gated behind a `touch-events` feature flag or run only in environments that support `TouchEvent` construction.

Touch interaction helpers (`touch_start(point)`, `touch_move(point)`, `touch_end()`, `advance_time(duration)`) are defined in the `TestHarness` API — see [15-test-harness.md](../testing/15-test-harness.md).

### 4.1 Long-Press

```rust
#[test]
fn context_menu_long_press_opens() {
    let harness = render(ContextMenu::new());
    harness.touch_start(point(100, 100));
    harness.advance_time(Duration::from_millis(500));
    assert!(harness.is_open());
    harness.touch_end();
}

#[test]
fn long_press_cancels_on_move() {
    let harness = render(ContextMenu::new());
    harness.touch_start(point(100, 100));
    harness.advance_time(Duration::from_millis(200));
    harness.touch_move(point(120, 120)); // Move beyond threshold
    harness.advance_time(Duration::from_millis(400));
    assert!(!harness.is_open()); // Cancelled
}
```

### 4.2 Touch vs Hover Disambiguation

```rust
#[test]
fn tooltip_no_show_on_touch() {
    let harness = render(Tooltip::new("Help text"));
    harness.touch_start_on_trigger();
    harness.touch_end();
    assert!(!harness.is_open()); // Tooltip should not open on touch
}

#[test]
fn hovercard_touch_opens_on_tap() {
    let harness = render(HoverCard::new());
    harness.tap_trigger();
    assert!(harness.is_open());
}
```

### 4.3 Multi-Touch

```rust
#[test]
fn slider_ignores_second_touch() {
    let harness = render(Slider::new(50.0));
    harness.touch_start(point(100, 50)); // First touch
    harness.touch_start(point(200, 50)); // Second touch
    harness.touch_move_first(point(150, 50));
    assert_ne!(harness.value(), 50.0); // First touch still tracked
}
```

### 4.4 Swipe Gestures

```rust
#[test]
fn drawer_swipe_to_close() {
    let harness = render(Drawer::new().placement(Placement::Left).open(true));
    harness.swipe(Direction::Left, 200.0);
    assert!(!harness.is_open());
}
```

## 5. Color Input Tests

Tests verifying color input components including ColorArea, ColorWheel, ColorSlider, and color format conversions.

### 5.1 ColorArea Pointer Interaction

```rust
#[test]
fn color_area_pointer_updates_hue_saturation() {
    let harness = render(ColorArea::new());

    // Click at center of area
    let area = harness.query_selector("[data-ars-color-area]").expect("color area element must exist");
    let center_x = area.offset_width() as f64 / 2.0;
    let center_y = area.offset_height() as f64 / 2.0;
    harness.pointer_down_at(center_x, center_y);

    let color = harness.current_color();
    assert!(color.saturation() > 0.0, "saturation must be non-zero after center click");
}
```

### 5.2 Color Format Round-Trips

```rust
#[test]
fn color_format_round_trip_hex_rgb_hsl() {
    let original_hex = "#3366cc";

    let rgb = Color::from_hex(original_hex).to_rgb();
    let hsl = rgb.to_hsl();
    let round_tripped = hsl.to_rgb().to_hex();

    assert_eq!(
        round_tripped.to_lowercase(),
        original_hex.to_lowercase(),
        "hex → rgb → hsl → rgb → hex must round-trip"
    );
}

#[test]
fn color_edge_cases_round_trip() {
    for hex in ["#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff"] {
        let round_tripped = Color::from_hex(hex).to_rgb().to_hsl().to_rgb().to_hex();
        assert_eq!(round_tripped.to_lowercase(), hex, "round-trip failed for {hex}");
    }
}
```

### 5.3 WCAG Contrast Ratio Calculation

```rust
#[test]
fn wcag_contrast_ratio_accuracy() {
    let white = Color::from_hex("#ffffff");
    let black = Color::from_hex("#000000");

    let ratio = white.contrast_ratio(&black);
    assert!((ratio - 21.0).abs() < 0.1, "white/black contrast must be ~21:1, got {ratio}");

    let mid_gray = Color::from_hex("#777777");
    let gray_ratio = white.contrast_ratio(&mid_gray);
    assert!(gray_ratio >= 4.48 && gray_ratio <= 4.50, "white/gray contrast mismatch");
}
```

### 5.4 ColorWheel Angle-to-Hue Mapping

```rust
#[test]
fn color_wheel_angle_to_hue() {
    let harness = render(ColorWheel::new());

    // Click at 0° (top/right) → hue ≈ 0
    harness.click_at_angle(0.0);
    assert!((harness.current_color().hue() - 0.0).abs() < 5.0);

    // Click at 120° → hue ≈ 120
    harness.click_at_angle(120.0);
    assert!((harness.current_color().hue() - 120.0).abs() < 5.0);

    // Click at 240° → hue ≈ 240
    harness.click_at_angle(240.0);
    assert!((harness.current_color().hue() - 240.0).abs() < 5.0);
}
```

### 5.5 ColorSlider Keyboard Increment/Decrement

```rust
#[test]
fn color_slider_keyboard_increment() {
    let harness = render(ColorSlider::new().channel(ColorChannel::Hue).value(180.0));

    harness.focus_thumb();
    harness.press_key(KeyboardKey::ArrowRight);
    assert!(harness.current_value() > 180.0, "ArrowRight must increment hue");

    harness.press_key(KeyboardKey::ArrowLeft);
    assert!((harness.current_value() - 180.0).abs() < f64::EPSILON, "ArrowLeft must decrement hue");
}
```

### 5.6 Color Channel Clamping

```rust
#[test]
fn color_channel_clamping() {
    // RGB channels clamp to 0-255
    let color = Color::from_rgb(300, -10, 128);
    assert_eq!(color.red(), 255);
    assert_eq!(color.green(), 0);
    assert_eq!(color.blue(), 128);

    // Hue clamps/wraps to 0-360
    let hsl = Color::from_hsl(400.0, 50.0, 50.0);
    assert!(hsl.hue() >= 0.0 && hsl.hue() <= 360.0);

    // Saturation/Lightness clamp to 0-100
    let hsl2 = Color::from_hsl(180.0, 150.0, -10.0);
    assert_eq!(hsl2.saturation(), 100.0);
    assert_eq!(hsl2.lightness(), 0.0);
}
```

---

## 6. File Upload / Drop Zone Tests

Tests verifying file upload and drag-and-drop zone behavior, including validation and progress tracking.

### 6.1 Drag-Over Visual State

```rust
#[test]
fn drop_zone_drag_over_visual_state() {
    let harness = render(DropZone::new());
    let zone = harness.query_selector("[data-ars-drop-zone]").expect("drop zone element must exist");

    assert!(zone.attr("data-ars-drop-target").is_none());

    harness.fire_drag_enter(&zone);
    assert_eq!(zone.attr("data-ars-drop-target"), Some("true"));

    harness.fire_drag_leave(&zone);
    assert!(zone.attr("data-ars-drop-target").is_none());
}
```

### 6.2 File Type Validation

```rust
#[test]
fn file_type_validation_accept_filter() {
    let rejected = Rc::new(Cell::new(false));
    let on_reject = {
        let rejected = rejected.clone();
        move |_| { rejected.set(true); }
    };

    let harness = render(
        DropZone::new()
            .accept(vec!["image/png", "image/jpeg"])
            .on_reject(on_reject)
    );

    // Drop a PDF — should be rejected
    harness.drop_files(vec![MockFile::new("doc.pdf", "application/pdf", 1024)]);
    assert!(rejected.get());
    assert_eq!(harness.accepted_files().len(), 0);
}

#[test]
fn file_type_validation_accepts_matching() {
    let harness = render(DropZone::new().accept(vec!["image/png", "image/jpeg"]));

    harness.drop_files(vec![MockFile::new("photo.png", "image/png", 2048)]);
    assert_eq!(harness.accepted_files().len(), 1);
}
```

### 6.3 Multiple File Selection

```rust
#[test]
fn multiple_file_selection() {
    let harness = render(DropZone::new().multiple(true));

    harness.drop_files(vec![
        MockFile::new("a.png", "image/png", 1024),
        MockFile::new("b.png", "image/png", 2048),
        MockFile::new("c.png", "image/png", 512),
    ]);

    assert_eq!(harness.accepted_files().len(), 3);
}

#[test]
fn single_mode_rejects_multiple_files() {
    let harness = render(DropZone::new().multiple(false));

    harness.drop_files(vec![
        MockFile::new("a.png", "image/png", 1024),
        MockFile::new("b.png", "image/png", 2048),
    ]);

    assert_eq!(harness.accepted_files().len(), 1);
}
```

### 6.4 File Size Validation

```rust
#[test]
fn file_size_validation() {
    let harness = render(DropZone::new().max_size(1_048_576)); // 1 MB

    harness.drop_files(vec![MockFile::new("big.bin", "application/octet-stream", 2_000_000)]);
    assert_eq!(harness.accepted_files().len(), 0);
    assert_eq!(harness.rejected_files().len(), 1);

    harness.drop_files(vec![MockFile::new("small.bin", "application/octet-stream", 500_000)]);
    assert_eq!(harness.accepted_files().len(), 1);
}
```

### 6.5 Directory Upload

```rust
#[test]
fn drop_zone_directory_upload() {
    let harness = render(DropZone::new().directory(true));

    harness.drop_directory(MockDirectory::new("photos", vec![
        MockFile::new("a.jpg", "image/jpeg", 1024),
        MockFile::new("b.jpg", "image/jpeg", 2048),
    ]));

    assert_eq!(harness.accepted_files().len(), 2);
}
```

### 6.6 Upload Progress Callback

```rust
#[test]
fn upload_progress_callback() {
    let progress_values = Rc::new(RefCell::new(Vec::new()));
    let on_progress = {
        let pv = progress_values.clone();
        move |p: f64| { pv.borrow_mut().push(p); }
    };

    let harness = render(FileUpload::new().on_progress(on_progress));
    harness.upload_file(MockFile::new("data.csv", "text/csv", 10_000));
    harness.advance_upload_to_completion();

    let values = progress_values.borrow();
    assert!(!values.is_empty());
    assert!(*values.last().expect("progress values must not be empty") >= 100.0, "progress must reach 100%");
    // Progress values should be monotonically increasing
    for w in values.windows(2) {
        assert!(w[1] >= w[0], "progress must be monotonically increasing");
    }
}
```

> **WASM safety:** Test code targeting `wasm32` must use `Rc<RefCell<>>` instead of `Arc<Mutex<>>`. `Mutex::lock()` can deadlock in single-threaded WASM if called reentrantly. `Arc` compiles but is unnecessary overhead.

---

## 7. Signature Pad / Canvas Input Tests

Tests verifying canvas-based input components including signature pads, image croppers, and QR code generators.

### 7.1 Pointer Drawing Creates Stroke Data

```rust
#[test]
fn signature_pad_pointer_drawing() {
    let harness = render(SignaturePad::new());

    assert!(harness.stroke_data().is_empty());

    // Simulate a stroke
    harness.pointer_down_at(50, 50);
    harness.pointer_move_to(100, 80);
    harness.pointer_move_to(150, 50);
    harness.pointer_up();

    assert!(!harness.stroke_data().is_empty(), "drawing must produce stroke data");
    assert!(harness.stroke_data().points().len() >= 3);
}
```

### 7.2 Clear Resets Canvas and Stroke Data

```rust
#[test]
fn signature_pad_clear() {
    let harness = render(SignaturePad::new());

    // Draw something
    harness.pointer_down_at(50, 50);
    harness.pointer_move_to(150, 50);
    harness.pointer_up();
    assert!(!harness.stroke_data().is_empty());

    harness.clear();

    assert!(harness.stroke_data().is_empty(), "clear must reset stroke data");
    assert!(harness.is_canvas_blank(), "clear must reset canvas pixels");
}
```

### 7.3 Save Exports Image Data

```rust
#[test]
fn signature_pad_export_png() {
    let harness = render(SignaturePad::new());

    harness.pointer_down_at(10, 10);
    harness.pointer_move_to(100, 100);
    harness.pointer_up();

    let png_data = harness.export(ExportFormat::Png);
    assert!(png_data.starts_with(&[0x89, b'P', b'N', b'G']), "export must produce valid PNG");
}

#[test]
fn signature_pad_export_svg() {
    let harness = render(SignaturePad::new());

    harness.pointer_down_at(10, 10);
    harness.pointer_move_to(100, 100);
    harness.pointer_up();

    let svg_data = harness.export_string(ExportFormat::Svg);
    assert!(svg_data.contains("<svg"), "export must produce valid SVG");
    assert!(svg_data.contains("<path"), "SVG must contain path elements");
}
```

### 7.4 ImageCropper Crop Area Selection

```rust
#[test]
fn image_cropper_crop_area() {
    let harness = render(ImageCropper::new().src("test.jpg"));

    harness.set_crop_area(CropRect { x: 10, y: 10, width: 200, height: 150 });

    let crop = harness.crop_area();
    assert_eq!(crop.x, 10);
    assert_eq!(crop.y, 10);
    assert_eq!(crop.width, 200);
    assert_eq!(crop.height, 150);
}

#[test]
fn image_cropper_aspect_ratio_constraint() {
    let harness = render(ImageCropper::new().src("test.jpg").aspect_ratio(16.0 / 9.0));

    harness.set_crop_area(CropRect { x: 0, y: 0, width: 320, height: 200 });

    let crop = harness.crop_area();
    let ratio = crop.width as f64 / crop.height as f64;
    assert!((ratio - 16.0 / 9.0).abs() < 0.01, "crop must respect aspect ratio constraint");
}
```

### 7.5 QRCode Renders with Valid Data

```rust
#[test]
fn qr_code_renders_with_data() {
    let harness = render(QrCode::new().value("https://example.com"));

    let svg = harness.query_selector("svg");
    assert!(svg.is_some(), "QR code must render an SVG element");
    assert!(harness.query_selector("svg rect").is_some(), "QR code SVG must contain modules");
}

#[test]
fn qr_code_updates_on_value_change() {
    let harness = render(QrCode::new().value("hello"));
    let svg1 = harness.query_selector("svg").expect("SVG element must exist").inner_html();

    harness.set_value("world");
    let svg2 = harness.query_selector("svg").expect("SVG element must exist after value change").inner_html();

    assert_ne!(svg1, svg2, "QR code must re-render when value changes");
}
```

### 7.6 Undo/Redo for Drawing Operations

```rust
#[test]
fn signature_pad_undo_redo() {
    let harness = render(SignaturePad::new());

    // Draw first stroke
    harness.pointer_down_at(10, 10);
    harness.pointer_move_to(50, 50);
    harness.pointer_up();
    let after_first = harness.stroke_count();

    // Draw second stroke
    harness.pointer_down_at(60, 60);
    harness.pointer_move_to(100, 100);
    harness.pointer_up();
    assert_eq!(harness.stroke_count(), after_first + 1);

    // Undo removes last stroke
    harness.undo();
    assert_eq!(harness.stroke_count(), after_first);

    // Redo restores it
    harness.redo();
    assert_eq!(harness.stroke_count(), after_first + 1);
}
```

---

## 8. Clipboard Operation Tests

Tests verifying clipboard read/write operations, permission handling, and fallback behavior.

### 8.1 Copy Writes to Clipboard

```rust
#[test]
fn copy_writes_to_clipboard() {
    let harness = render(CopyButton::new().value("Hello, world!"));

    harness.click();

    assert_eq!(harness.clipboard_text(), "Hello, world!");
}
```

### 8.2 Paste Reads from Clipboard

```rust
#[test]
fn paste_reads_from_clipboard() {
    let harness = render(PasteTarget::new());

    harness.set_clipboard_text("pasted content");
    harness.paste();

    assert_eq!(harness.received_text(), "pasted content");
}
```

### 8.3 Clipboard Permission Denied Handling

```rust
/// Mock a browser permission grant for testing.
/// Intercepts the Permissions API to return "granted" for the specified permission.
fn mock_permission_grant(permission: &str) {
    js_sys::eval(&format!(
        "navigator.permissions.query = (desc) => \
         Promise.resolve({{ state: desc.name === '{}' ? 'granted' : 'prompt' }})",
        permission
    )).expect("mock permission grant injection");
}

/// Test utility for APIs requiring user permissions (clipboard, notifications).
///
/// **Browser security context:** These APIs require either:
/// - HTTPS or localhost origin
/// - User gesture (click) preceding the API call
/// - Permissions API grant
///
/// In CI (headless): Use `--unsafely-treat-insecure-origin-as-secure` flag
/// or mock the Permissions API.
fn render_with_permissions(component: impl IntoView, permissions: &[&str]) -> TestHarness {
    // Grant specified permissions via Permissions API mock
    for perm in permissions {
        mock_permission_grant(perm);
    }
    TestHarness::mount(component)
}

#[test]
fn clipboard_permission_denied_handling() {
    let harness = render_with_permissions(
        CopyButton::new().value("secret"),
        &["clipboard-write:denied"],
    );

    harness.click();

    assert!(
        harness.input_attr("aria-invalid") == Some("true".into()),
        "must surface error when clipboard permission denied"
    );
    assert!(harness.query_selector("[data-ars-copied]").is_none());
}
```

### 8.4 Copied Feedback State with Auto-Reset

```rust
#[test]
fn copied_feedback_auto_reset() {
    let harness = render(CopyButton::new().value("data").reset_delay(Duration::from_millis(200)));

    harness.click();
    assert!(harness.query_selector("[data-ars-copied]").is_some());

    harness.advance_time(Duration::from_millis(100));
    assert!(harness.query_selector("[data-ars-copied]").is_some(), "must still show copied state");

    harness.advance_time(Duration::from_millis(150));
    assert!(harness.query_selector("[data-ars-copied]").is_none(), "copied state must auto-reset");
}
```

### 8.5 Clipboard with Rich Text Content

```rust
#[test]
fn clipboard_rich_text_html() {
    let harness = render(CopyButton::new()
        .value_html("<strong>Bold</strong> and <em>italic</em>")
    );

    harness.click();

    let clip = harness.clipboard_contents();
    assert_eq!(clip.plain_text(), "Bold and italic");
    assert!(clip.html().contains("<strong>Bold</strong>"));
}
```

### 8.6 Fallback for Browsers Without Clipboard API

```rust
#[test]
fn clipboard_fallback_without_api() {
    let harness = render_with_features(
        CopyButton::new().value("fallback test"),
        Features { clipboard_api: false },
    );

    harness.click();

    // Should fall back to execCommand or textarea copy
    assert_eq!(harness.clipboard_text(), "fallback test");
}
```

---

## 9. Animation and Transition Testing

Components using Presence for enter/exit animations MUST have dedicated animation tests.

### 9.1 Mock Events

- Tests dispatch synthetic `animationend` and `transitionend` events to simulate animation completion.
- Use `dispatchEvent(new AnimationEvent('animationend', { animationName: '...' }))`.

### 9.2 Required Test Scenarios

```rust
#[test]
fn entry_animation_completes_to_mounted() {
    let harness = render(PresenceWrapper::new().present(true));
    harness.dispatch_animation_event("animationend");
    assert_eq!(harness.presence_state(), "Mounted");
}

#[test]
fn exit_animation_completes_to_unmounted() {
    let harness = render(PresenceWrapper::new().present(true));
    harness.set_present(false);
    harness.dispatch_animation_event("animationend");
    assert!(harness.is_unmounted() || harness.presence_state() == "Unmounted");
}

#[test]
fn rapid_open_close_no_stale_state() {
    let harness = render(PresenceWrapper::new().present(false));
    harness.set_present(true);
    harness.set_present(false);
    harness.set_present(true);
    harness.dispatch_animation_event("animationend");
    assert_eq!(harness.presence_state(), "Mounted");
}

#[test]
fn unmount_on_exit_waits_for_animation() {
    let harness = render(PresenceWrapper::new().present(true).unmount_on_exit(true));
    harness.set_present(false);
    assert!(harness.is_in_dom(), "must remain in DOM until animation completes");
    harness.dispatch_animation_event("animationend");
    assert!(!harness.is_in_dom(), "must unmount after animationend");
}

#[test]
fn safety_timeout_fires_if_no_animation_event() {
    let harness = render(PresenceWrapper::new().present(true).unmount_on_exit(true));
    harness.set_present(false);
    harness.advance_time(Duration::from_millis(5000));
    assert!(!harness.is_in_dom(), "safety timeout must force unmount");
}

#[test]
fn lazy_mount_renders_before_animation() {
    let harness = render(PresenceWrapper::new().present(false).lazy_mount(true));
    assert!(!harness.is_in_dom());
    harness.set_present(true);
    assert!(harness.is_in_dom(), "must render before animation starts");
    assert_eq!(harness.presence_state(), "Mounting");
}
```

### 9.3 Timing Assertions

- Focus MUST NOT move until entry animation starts (`animationstart`).
- `data-ars-state` transitions match Presence state machine.

---

## 10. Content Security Testing

Components accepting user-provided text for labels, descriptions, and error messages MUST be tested for content injection.

### 10.1 XSS Fuzzing

Property-based tests using `proptest` with HTML/script payloads as label/description inputs.

```rust
use proptest::prelude::*;

const XSS_PAYLOADS: &[&str] = &[
    "<script>alert(1)</script>",
    "\" onmouseover=\"alert(1)\"",
    "javascript:void(0)",
    "<img src=x onerror=alert(1)>",
    "<svg/onload=alert(1)>",
];

proptest! {
    #[test]
    fn label_escapes_html(payload in prop::sample::select(XSS_PAYLOADS)) {
        let harness = render(TextField::new().label(payload));
        let label_html = harness.query_selector("label").expect("label element must exist").inner_html();
        assert!(!label_html.contains("<script>"), "script tags must be escaped");
        assert!(!label_html.contains("onerror="), "event handlers must be escaped");
        assert!(!label_html.contains("onmouseover="), "event handlers must be escaped");
    }
}
```

### 10.2 ARIA Label Escaping

- `aria-label` values MUST NOT contain unescaped HTML.
- Test: set label to `<b>Bold</b>`, verify `aria-label="<b>Bold</b>"` (escaped, not rendered).

```rust
#[test]
fn aria_label_escapes_html() {
    let harness = render(Button::new().aria_label("<b>Bold</b>"));
    let attr = harness.root().get_attribute("aria-label").expect("aria-label attribute must exist");
    assert_eq!(attr, "<b>Bold</b>", "aria-label must contain escaped HTML, not rendered HTML");
}
```

### 10.3 Sanitization Boundary

- ars-core treats all string inputs as plain text. No HTML parsing or rendering.
- Adapters (Leptos, Dioxus) MUST use their framework's text interpolation (which auto-escapes) rather than `innerHTML`.
- Test: verify no adapter uses `innerHTML`, `dangerously_set_inner_html`, or equivalent for user-provided content.

```rust
#[test]
fn no_inner_html_for_user_content() {
    // Static analysis / grep test: ensure no adapter source file uses innerHTML
    // for user-provided label, description, or error message content.
    // Requires: glob = "0.3" as dev-dependency
    let adapter_sources = glob("crates/ars-{leptos,dioxus}/src/**/*.rs").expect("glob pattern must be valid");
    for file in adapter_sources {
        let content = std::fs::read_to_string(&file).expect("adapter source file must be readable");
        assert!(
            !content.contains("inner_html") && !content.contains("dangerously_set_inner_html"),
            "File {:?} must not use innerHTML for user-provided content",
            file,
        );
    }
}
```

---

## 11. Performance Benchmarks

The "zero-cost abstractions" claim requires empirical validation via benchmarks.

### 11.1 Micro-Benchmarks with `criterion.rs`

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_transition_throughput(c: &mut Criterion) {
    let props = toggle::Props::default();
    let (state, ctx) = toggle::Machine::init(&props);

    c.bench_function("toggle::transition", |b| {
        b.iter(|| {
            black_box(toggle::Machine::transition(
                &state,
                &toggle::Event::Toggle,
                &ctx,
                &props,
            ))
        })
    });
}

fn bench_init(c: &mut Criterion) {
    c.bench_function("dialog::init", |b| {
        let props = dialog::Props::default();
        b.iter(|| black_box(dialog::Machine::init(&props)))
    });
}

fn bench_connect(c: &mut Criterion) {
    let props = button::Props::default();
    let (state, ctx) = button::Machine::init(&props);
    let send = |_: button::Event| {};

    c.bench_function("button::connect", |b| {
        b.iter(|| black_box(button::Machine::connect(&state, &ctx, &props, &send)))
    });
}

criterion_group!(benches, bench_transition_throughput, bench_init, bench_connect);
criterion_main!(benches);
```

### 11.2 Macro-Benchmarks

```rust
fn bench_mount_unmount_cycle(c: &mut Criterion) {
    c.bench_function("dialog::mount_unmount_100x", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let mut svc = Service::<dialog::Machine>::new(dialog::Props::default());
                svc.send(dialog::Event::Open);
                svc.send(dialog::Event::Close);
            }
        })
    });
}

fn bench_large_collection(c: &mut Criterion) {
    let collection: StaticCollection<select::Item> = (0..1000)
        .map(|i| (Key::from(format!("{i}")), select::Item { label: format!("Item {i}") }))
        .collect();

    c.bench_function("select::1000_items_open_typeahead", |b| {
        b.iter(|| {
            let props = select::Props::default();
            let mut svc = Service::<select::Machine>::new(props);
            svc.send(select::Event::UpdateItems(collection.clone()));
            svc.send(select::Event::Open);
            svc.send(select::Event::TypeaheadSearch('I', 0));
            black_box(&svc.context().highlighted_key);
        })
    });
}
```

### 11.3 Memory Profiling

- Use `jemalloc` with `MALLOC_CONF="prof:true"` for heap profiling during benchmarks.
- Track allocations per `init()`, `transition()`, and `connect()` call.
- Set baseline thresholds and fail CI if allocations exceed 2x the baseline.
- Run `cargo bench` on every PR; store results in `target/criterion/` for trend analysis.

### 11.4 Benchmark Categories: Core vs Adapter Overhead

Adapter overhead must be measured separately from core logic to identify whether performance bottlenecks originate in state machine internals or framework integration layers.

#### 11.4.1 Core Benchmarks

Pure state machine performance with no adapter or DOM involvement.

| Benchmark                 | Description                                                                  | Threshold              |
| ------------------------- | ---------------------------------------------------------------------------- | ---------------------- |
| `transition()` throughput | Single state machine transition with context update                          | < 1μs per transition   |
| Event processing cycle    | `send()` → `transition()` → effect collection → context update               | < 10μs per event cycle |
| Validation logic          | Synchronous field validation (e.g., `validate_required`, `validate_pattern`) | < 10μs per event cycle |
| `init()` cold start       | Machine initialization with default props                                    | < 1μs per transition   |
| `connect()` AttrMap build | Full attribute map generation from state + context                           | < 10μs per event cycle |

```rust
fn bench_core_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("core");

    group.bench_function("toggle::transition", |b| {
        let props = toggle::Props::default();
        let (state, ctx) = toggle::Machine::init(&props);
        b.iter(|| black_box(toggle::Machine::transition(
            &state, &toggle::Event::Toggle, &ctx, &props,
        )))
    });

    group.bench_function("select::event_cycle", |b| {
        b.iter(|| {
            let mut svc = Service::<select::Machine>::new(select::Props::default());
            svc.send(select::Event::Open);
            black_box(svc.state());
        })
    });

    group.finish();
}
```

#### 11.4.2 Adapter Benchmarks

Framework-specific overhead measured in isolation from core logic.

| Benchmark               | Description                                                       | Threshold           |
| ----------------------- | ----------------------------------------------------------------- | ------------------- |
| Signal propagation      | Time from `send()` to reactive signal update in adapter           | < 100μs mount       |
| DOM reconciliation      | Diffing and patching DOM after state change                       | < 50μs update       |
| Component mount         | Full component mount including signal creation and initial render | < 100μs mount       |
| Component unmount       | Teardown including effect cleanup and signal disposal             | < 50μs update       |
| Frame budget compliance | Total time from user event to painted frame                       | < 16ms frame budget |

```rust
fn bench_adapter_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("adapter");

    group.bench_function("leptos::mount_toggle", |b| {
        b.iter(|| {
            // Note: Leptos 0.8.x API — verify signal creation syntax against docs.rs for your version.
            // In Leptos 0.7+, `signal()` returns (ReadSignal, WriteSignal).
            // In Leptos 0.8+, use `RwSignal::new()` or `signal()` from `leptos::prelude::*`.
            let (count, set_count) = leptos::signal(0);
            set_count.set(1);
            assert_eq!(count.get(), 1);
        });
    });

    group.bench_function("leptos::update_signal_propagation", |b| {
        let (pressed, set_pressed) = leptos::signal(false);
        b.iter(|| {
            set_pressed.update(|v| *v = !*v);
            black_box(pressed.get());
        });
    });

    group.finish();
}
```

#### 11.4.3 Integration Benchmarks

End-to-end latency from simulated user event to final DOM state.

| Benchmark           | Description                                            | Threshold |
| ------------------- | ------------------------------------------------------ | --------- |
| Simple interaction  | Click → state change → DOM update (e.g., toggle press) | < 5ms     |
| Complex interaction | Open select → filter → highlight → commit              | < 5ms     |
| Keyboard navigation | Arrow key → highlight move → scroll → ARIA update      | < 5ms     |

```rust
fn bench_integration_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("integration");

    group.bench_function("toggle::click_to_dom", |b| {
        b.iter(|| {
            // Leptos 0.8: no explicit runtime creation needed
            let _view = leptos::view! { <Toggle pressed=false /> };
            // Simulate click event through full pipeline
            simulate_click(&_view);
            let attrs = collect_dom_attributes(&_view);
            black_box(attrs.get("aria-pressed"));
        })
    });

    group.finish();
}
```

#### 11.4.4 CI Threshold Enforcement

```toml
# In Cargo.toml or a dedicated bench config
[benchmark.thresholds]
core_transition_us = 1
core_event_cycle_us = 10
adapter_mount_us = 100
adapter_update_us = 50
adapter_frame_ms = 16
integration_simple_ms = 5
```

When a benchmark exceeds its threshold, CI must emit a warning. When it exceeds 2x the threshold, CI must fail the run. Track per-category trends separately so that a core regression is not masked by adapter improvements (or vice versa).
