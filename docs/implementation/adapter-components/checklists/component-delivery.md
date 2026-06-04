# Adapter Component Delivery Checklist

Use this checklist while implementing. It is not a substitute for reading the
workflow docs.

## Before Code

- [ ] Assigned issue read.
- [ ] Issue moved to In Progress.
- [ ] Dependency checks run for Leptos and Dioxus.
- [ ] Agnostic and adapter specs read.
- [ ] Adapter foundation specs read.
- [ ] `adapter-contract.md` read.
- [ ] `examples/widgets-ownership.md` read.
- [ ] Framework skills loaded when touching Leptos/Dioxus.
- [ ] Counterpart outcome matrix written from live browser review.
- [ ] Counterpart outcome matrix records primary and fallback sources.
- [ ] `playwright-cli` reference/local browser evidence plan written.

## Adapter Code

- [ ] Component module added or updated.
- [ ] Category module wired.
- [ ] `lib.rs` category export updated if needed.
- [ ] Feature wiring updated if needed.
- [ ] Prelude exports are symmetric.
- [ ] Prop-facing config types are re-exported or aliased.
- [ ] Semantic attrs come from the agnostic API.
- [ ] Consumer styling is forwarded.
- [ ] `StoredValue` / `CopyValue` used for shared captured values.

## Tests And Examples

- [ ] Adapter SSR/unit tests added.
- [ ] Wasm tests added for browser behavior.
- [ ] E2E category aggregators updated for both adapters.
- [ ] E2E component fixture modules added for both adapters.
- [ ] E2E harness has one test per feature axis.
- [ ] E2E matrix entry covers every axis or records N/A.
- [ ] Axe runs across visible states.
- [ ] Computed visual assertions cover visible states.
- [ ] Widgets examples updated in all six crates.
- [ ] Widget smoke covers counterpart UX states.
- [ ] Browser evidence compares counterpart and local widgets pages.

## Closeout

- [ ] Spec drift fixed in same PR.
- [ ] Focused checks pass.
- [ ] `cargo xtask lint adapter-parity` passes without component skips.
- [ ] `post-implementation-audit` completed and findings fixed.
- [ ] Results presented before commit.
- [ ] User approval received before commit/push.
- [ ] `cargo xci-fast` passes before push.
- [ ] PR opened with auto-close keyword and counterpart outcome matrix.
- [ ] PR body includes browser evidence paths and parity status.
- [ ] `waiting-for-codex-review` loop completed after every push.
