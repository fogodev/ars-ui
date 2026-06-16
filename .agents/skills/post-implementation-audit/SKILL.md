---
name: post-implementation-audit
description: MANDATORY after every implementation task — runs three sequential audits on the new code (spec/impl drift, iterative "anything missing?", test coverage) and lands every finding in the same PR before the user-review step. Invoke as soon as an implementation task's named tests pass, OR when any of these phrases appears: "ready to present", "implementation is complete", "named tests pass", "before opening the PR", "ready for review". Skipping this audit leaves spec drift, untested defensive code, missing conventions, and silent contract violations in the merged PR.
---

# Post-implementation audit

This skill is the bridge between _"the named tests pass"_ and _"the user reviews the diff"_. It exists because the initial implementation of any task is usually correct on its surface acceptance criteria but quietly drifts from the spec, leaves untested defensive code, ships APIs that diverge from convention, or carries a fold-vs-lowercase mistake that won't surface until production. Running these audits before user-review collapses three reviewer round-trips into one.

The skill is mandatory per CLAUDE.md "Development Workflow" step 7 — run it after implementing any user-requested task (feature, bug fix, refactor) and before presenting the final result for user review.

## Quick flow

The audit is strictly linear — no branching, just a loop in Phase 2:

```
Phase 1 (spec/impl drift audit)
   └─→ land all Phase 1 findings
        └─→ Phase 2 (iterative "anything missing?")
              ├─→ Round 1: surface findings → land all
              ├─→ Round 2: surface findings → land all
              └─→ keep looping until 2 consecutive clean rounds (minimum 2 rounds total)
                   └─→ Phase 3 (test coverage)
                        └─→ land all Phase 3 findings
                             └─→ Adapter parity loop (adapter component tasks only)
                                  └─→ Final verification suite
                                  └─→ Hand off to user (CLAUDE.md step 8)
```

The body below walks through each phase in order. Read top-to-bottom; the structure mirrors the execution order.

## Operating principle: no deferral

**Every recommendation each audit phase produces lands in _this_ PR.** Don't open follow-up issues. Don't TODO-flag. Don't promise a future PR or a future sprint. Don't quietly skip a finding because it feels like scope creep.

If a finding is genuinely out of scope (e.g., requires a workspace-wide refactor touching 50+ files), surface it explicitly to the user with that reasoning visible — but the default is to land everything. This constraint is what makes the audit valuable: a deferred finding is a guarantee that the original implementation's bias survives into `main`, where it costs ten times as much to undo.

The user will tell you "land it all". Anticipate that, and just do it.

## Three sequential phases

Run the phases **in order**. Each phase fully completes (audit → land all findings → re-verify) before the next begins.

---

### Phase 1 — spec/implementation drift audit

Compare the implementation against the authoritative spec (`spec/components/<category>/<component>.md` plus any foundation specs it transitively depends on) section-by-section. For every drift, propose the **best outcome independent of which side is currently right**. Sometimes the spec wins, sometimes the impl wins, sometimes the right answer is neither.

The canonical entry-prompt for this phase (use this wording when the user asks you to start the audit explicitly):

> "Now let's do an audit on how well our new code follows our spec. Do an in-depth check for spec drift and suggest recommendations for each finding. In those recommendations, disregard both the spec and the current implementation as defaults — the recommendation should be the best outcome possible. Sometimes that's the spec, sometimes the implementation, sometimes a mix, sometimes a totally different thing."

#### Precondition: both files must exist

Before reading anything, verify the touched implementation has a corresponding spec file at `spec/components/<category>/<name>.md` (or the relevant `spec/foundation/` path for foundation-layer changes). If the spec is missing, **stop the audit and surface this to the user** — an unspecced module is a planning failure that should be resolved (write the spec, or split the change) before auditing, not papered over by this skill. Likewise, if the implementation file referenced by the issue is missing or empty, surface that as a precondition failure rather than auditing against an empty diff.

#### Procedure

1. Read the spec file end-to-end (use `Read` with no offset/limit so you have the whole thing).
2. Read the implementation file end-to-end (same).
3. Walk every public surface declared in the spec — every struct, enum, function, method, derive list, default value, constant, public re-export. For each:
    - Does the spec define it? Does the impl match exactly?
    - Are there extras in the impl not in the spec? Are they justified?
    - Are there spec items the impl skipped? Are they critical?
    - **Does the spec's prose match the impl's actual behavior?** This is the silent-contract-violation class — words that mean something specific in the spec but were implemented as something close-but-different (e.g., "case folding" in the spec but `to_lowercase` in the impl).
4. For each finding, write up:
    - What the spec says, with line numbers
    - What the impl does, with line numbers
    - Why it drifted (often a tradeoff the implementer made under acceptance-criteria pressure)
    - **Best-outcome recommendation** — independent of which side is currently right
5. Group findings by severity (High / Medium / Low) and present as a table to the user.
6. Land all of them. Update the spec where the impl is right, update the impl where the spec is right, do both where neither was right, change _only_ the spec wording (and add tests proving the new wording) when the contract was subtly wrong all along.

#### Special case: both the spec and the implementation are wrong

A small fraction of drift findings are _not_ "one side is right, the other drifted" — both sides are incorrect, incomplete, or built on a flawed premise. This is rare but real (the task #204 eszett case is an example: the spec implied `ß ↔ ss` equivalence, the impl used `to_lowercase`, but neither matched the actual TR21 case-folding semantics needed). Handle this case differently from routine drift:

1. **Escalate to the user via `AskUserQuestion`** — don't unilaterally pick a "best outcome" when no precedent exists. A both-wrong case is a real design decision, not a routine drift fix; the user owns the call.
2. **Once the user picks a direction**, land _all three_ together in this same PR: the new spec wording, the new implementation, and the tests proving the new design. Skipping any of the three leaves a different kind of drift behind.
3. **Write the test first** (TDD per CLAUDE.md), watch it fail under the old code, then change the impl, then update the spec to describe what the new tests assert.

#### Output table format

| Finding ID | Severity | What spec says | What impl does | Best outcome | Where to land              |
| ---------- | -------- | -------------- | -------------- | ------------ | -------------------------- |
| H1         | High     | …              | …              | …            | spec §X / impl `path:line` |

End with a disposition summary: _"N spec-only changes, M impl-only changes, K both, Q status-quo-was-correct."_

---

### Phase 2 — iterative "anything else missing?" passes

Once Phase 1 findings are landed, the next blind spot is the surface Phase 1 didn't look at: adapter specs, feature flags, prelude exports, framework wiring, foundation patterns that deserve promotion, catalog/manifest entries. This phase is **iterative** — keep asking "anything else?" until two consecutive rounds find nothing new.

The canonical entry-prompt (one per round):

> "Any other improvement that we might still be missing?"

#### Surfaces to actively check each round

This is a non-exhaustive checklist. Each round, walk through it explicitly:

- **Adapter specs** at `spec/leptos-components/<category>/<component>.md` and `spec/dioxus-components/<category>/<component>.md` — do they describe the new core surface accurately? Are their canonical implementation sketches up-to-date with the actual `Api` / function signatures? Do they reference the right helper methods (e.g., `Api::root_attrs()`) instead of hardcoded data attribute strings?
- **Adapter parity workflow** at `docs/implementation/adapter-component-delivery.md` and linked files — for adapter component work, did the task produce and update a reference-exploration sketch, outcome matrix, browser evidence, widgets evidence, i18n/a11y mapping, and parity audit loop?
- **Retrofit audit workflow** — if the task updates an older adapter component
  to current conventions, did it create a fresh audit issue, compare against
  the current gold-standard component, refresh old browser/E2E evidence, and
  run stale-symbol scans for any public primitive rename?
- **Adapter feature wiring** in `crates/ars-leptos/Cargo.toml` and `crates/ars-dioxus/Cargo.toml` — does the adapter's `icu4x` / `web-intl` feature need to enable any new `ars-components/<feature>` flag for the new code to be reachable?
- **Adapter `prelude.rs`** — does the new public API need to flow through? (Only if an adapter wrapper exists for the new component; if not, defer to the adapter task.)
- **Dioxus Rules of Hooks** — if any changed file is under `crates/ars-dioxus/src`, verify hooks run in a stable top-level order. Do not hide hooks in `unwrap_or_else`, `map_or_else`, conditionals, loops, iterator adapters, nested closures, or early-return branches. Run the fallback-hook probe against the changed Dioxus files, not the whole crate:

    ```bash
    rg 'unwrap_or_else\(\|\| use_|map_or_else\([^\n]*use_' <changed-dioxus-files>
    ```

    Treat every hit as suspicious until proven safe, then manually review the same changed files for hooks inside conditionals, loops, iterator adapters, nested closures, and early-return branches. The safe fallback-ID pattern is:

    ```rust
    let generated_id = use_stable_id("field");
    let id = props.id.unwrap_or(generated_id);
    ```

- **Target capability classification** — if the changed adapter code uses a
  browser-owned API or semantic such as constraint validation, focus,
  selection ranges, layout measurement, clipboard, drag data, or file inputs,
  verify the spec/sketch records which behavior is `TypedWebDom`,
  `WebViewBridge`, `ServerOrSsr`, or `NoDomNative`. Do not accept prose that
  claims universal native behavior when a target only has a fallback.

- **Dioxus global attributes** — if a changed Dioxus component has
  `#[props(extends = GlobalAttributes)]`, verify it does not also expose
  explicit root `class`, `style`, `data-*`, `lang`, `tabindex`, or extra
  `aria-*` props. Those attrs should flow through `attrs: Vec<Attribute>` and
  merge with agnostic root attrs. Keep an explicit prop only for semantic
  component data, non-root part attrs, typed HTML vocabularies, or documented
  component-owned precedence/validation. Add or keep a test proving `class:`
  and `style:` still work through global attrs.
- **Compound part styling** — for multi-part components with visible internal
  anatomy, verify the adapter exposes public compound parts for stylable
  anatomy instead of adding repeated root-level `*_class` / `*_style` props.
  Dioxus parts should use `GlobalAttributes`; Leptos parts should use reactive
  `TextProp` class/style until a broader Leptos global-attrs surface exists.
  Tailwind widget examples must style those public parts or use Tailwind
  arbitrary variants over `data-ars-*` anatomy, not raw Rust string CSS.
- **Styled source-template boundary** — verify checked-in closed-anatomy styled
  component templates live in `ars-leptos-components` /
  `ars-dioxus-components`, not in `ars-leptos` / `ars-dioxus`. Adapter crates
  should expose unstyled primitives and core exports; styled crates should
  compose those primitives and expose CSS and Tailwind variants when the
  widget/demo contract needs both. Treat the styled crates as reference source
  for the future `ars-ui` CLI, which copies editable component source into user
  projects; do not accept package-only customization workarounds when copied
  source or public primitives are the right boundary.
- **Styled template layout** — verify styled source templates are organized
  category-first under `src/<category>/<component>/`, with adjacent `.css`
  files for CSS variants. Do not accept top-level variant-first trees such as
  `css::checkbox` or `tailwind::checkbox`; those crates stage source for the
  installer, so category-first paths should be the only public shape.
- **CSS template documentation** — verify CSS variant files include comments
  explaining the component parts and state selectors they style, so copied
  component source is understandable and safely customizable.
- **Tailwind template editability** — verify Tailwind source templates keep
  class strings inline in the rendered `view!` / `rsx!` markup rather than
  hiding them behind `const` identifiers. Inline classes preserve copied-source
  editability and Tailwind-aware editor completion/canonical diagnostics.
- **Styled template import boundary** — verify copied-source styled template
  Rust files import only `ars_leptos::prelude::*` or
  `ars_dioxus::prelude::*`. Do not accept direct imports from `leptos`,
  `dioxus`, `ars_forms`, or deep adapter/foundation modules in those templates;
  re-export user-facing helpers and prop types from the adapter prelude first.
- **Prop conversion ergonomics** — verify user-facing callback, signal, text,
  and view props use `#[prop(into)]` / `#[props(into)]` where supported, and
  examples pass closures, signals, translated memos, elements, and view
  closures directly instead of noisy `Callback::new`, `Signal::derive`,
  `ViewFn::from`, `.into()`, or `EventHandler` wrappers unless a wrapper value
  is intentionally reused or inference requires it.
- **Workspace `spec/manifest.toml`** and **`foundation/02-component-catalog.md`** — is the new component registered and statused? (For brand-new components only.)
- **Foundation specs** (`spec/foundation/00-*` through `spec/foundation/11-*`) — does the new code expose a missing shared abstraction worth promoting? CLAUDE.md `spec/CLAUDE.md` says: _"If an adapter-specific implementation exposes a missing shared abstraction, promote that abstraction into the appropriate foundation/shared spec."_
- **Adapter semantic boundary** — inspect private helper functions in changed
  adapter files and duplicated Leptos/Dioxus branches. Any helper that can be
  unit-tested without framework types, browser handles, DOM refs, or renderer
  APIs belongs in `ars-components` or a shared foundation crate. Do not accept
  duplicated helpers that return or consume component `State`, `Event`, `Props`,
  `AttrMap`, ids, validation errors, keyboard decisions, disabled/readonly
  rules, or ARIA relationship decisions unless they are marked as adapter
  rendering/framework glue with a concrete reason. Adapter-local extension
  traits over agnostic `Api`, `Props`, `State`, or `Event` are drift; add the
  method to the agnostic API instead.
- **E2E blocker classification** — if an E2E command failed, verify the result
  is classified as `ComponentAssertionFailed`, `HarnessSetupFailed`, or
  `EvidenceMissing`. Only reached component assertions count as component
  behavior evidence. Harness/setup failures leave the affected outcome rows
  unverified until fixed or explicitly handed off as partial.
- **New crate dependencies** — was the user notified per CLAUDE.md's "Do not add a new dependency crate without explicit user approval first" rule? Did the implementation add `i18n`-style feature flags that warrant a CLAUDE.md note?
- **`.cargo/mutants.toml`** — are any new equivalent mutations documented with justifications? (Phase 3 will validate this; surface it here if a mutation that _should_ be equivalent isn't documented yet.)

#### Termination

Stop when:

- Two consecutive rounds surface no new findings, **OR**
- Three rounds have been completed and the remaining findings are honestly out-of-scope (genuine workspace-wide refactors that would derail this PR).

Do **not** stop after one clean round — the second round nearly always surfaces items the first ignored because they felt out-of-scope. With the "no deferral" rule active, they're back in scope.

#### Output per round

A list of findings with the same "best outcome" framing as Phase 1, plus an explicit statement at the end: _"That's everything I can find this round. Want me to do another?"_ — knowing the answer will be yes for at least one more round.

---

### Phase 3 — test coverage audit

By this point the code is convention-aligned and well-specced. Now check the **test surface** — both depth (line / branch / mutation coverage) and breadth (does every test type that _should_ exist for this kind of change exist?).

The canonical entry-prompt:

> "How good is our test coverage here, by looking at the code paths, checking the coverage report and considering the many kinds of tests that we are intending to have (including snapshot tests, proptests, and e2e tests)? Can it be improved?"

#### Test types and what each catches

| Test type                                            | What it catches                                          | How to assess for the new code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Unit tests** (inline `#[cfg(test)]`)               | Logic + edge cases on the named acceptance criteria      | List every public API + every edge case the spec describes; cross-reference with the test list                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| **Snapshot tests** (insta)                           | `AttrMap` / chunk-output shape regressions               | Check coverage of every anatomy part and every output-affecting prop / state / context branch                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| **Property tests** (proptest)                        | Invariant violations across the input space              | Check that invariants are non-trivial (roundtrip, idempotence, no-adjacent-X, etc.) and the input strategy is broad enough to actually exercise edge locales / multi-byte / large inputs                                                                                                                                                                                                                                                                                                                                         |
| **Mutation tests** (`cargo xmutants`)                | Tests that _exist_ but don't actually constrain behavior | **Not run as part of this audit.** Owned by the Nightly CI `mutation-testing` job; `MISSED` mutants are triaged in a follow-up, never as a pre-PR gate. Use the coverage report + breadth audit to judge test quality here.                                                                                                                                                                                                                                                                                                      |
| **Doc tests**                                        | Public-API examples broken by future changes             | Check `///` blocks for at least one `# Examples` block on the canonical entry point                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| **Spec-conformance tests**                           | Anatomy drift between spec and impl                      | Check `crates/ars-components/tests/spec_conformance/*.rs` for an entry covering the new component's `Part` enum and public API/attribute contract; this is mandatory for every new framework-agnostic component                                                                                                                                                                                                                                                                                                                  |
| **Code coverage** (cargo llvm-cov + xtask wasm path) | Unreachable / under-tested code; threshold drift         | Use the right command for the crate (see "Procedure" below) — bare `cargo llvm-cov` does **not** measure adapter wasm code paths. Always finish with `cargo xtask coverage check-all --file <lcov>` to enforce the same thresholds CI does.                                                                                                                                                                                                                                                                                      |
| **Adapter wasm browser tests** (wasm-bindgen-test)   | Framework adapter wiring that SSR cannot prove           | Required when the change touches `ars-leptos`, `ars-dioxus`, `ars-dom`, or `ars-i18n` (`web-intl` path) and behavior depends on a browser runtime. These should stay focused: DOM-mounted attrs, generated ids, relationship attrs, callback dispatch, form event prevention, focus/keyboard/pointer paths, reactive DOM updates, and mount cleanup. They are not full E2E parity proof. Treat coverage from the wasm path (`cargo xtask coverage wasm`) as the source of truth for these — not the host-target llvm-cov number. |
| **E2E tests**                                        | User-visible workflows and counterpart outcome parity    | Required for adapter components with browser-observable behavior. E2E owns complete workflows, Leptos/Dioxus parity, computed visual feedback, axe-clean reached states, styled fixture/widget behavior, every supported reference outcome axis, and every UX/browser-review complaint from the session. A wasm test can support a parity row, but it cannot be the only proof unless the outcome is strictly low-level adapter/browser wiring.                                                                                  |

For adapter components, add a dedicated composition integration audit before
calling the test surface complete. If the component consumes `Form`, `Field`,
`Fieldset`, provider, group, collection, or overlay context, verify tests cover
the composed outcome with those foundations. For form controls, `Form` and
`Fieldset` are mandatory: submit serialization, reset, inherited
disabled/readonly/invalid, matching validation errors by `name`, unmatched
errors ignored, and description/error relationship ordering must be covered
across SSR/unit plus wasm or E2E where browser behavior is involved.

For adapter E2E audits, build a complaint-to-regression map before calling the
surface complete. Each browser comment or user-visible complaint must point to
one harness assertion that would have failed before the fix, or to a documented
`NotApplicable` / `IntentionallyDifferent` matrix row. For forms and validation
surfaces, check that E2E covers valid state, each invalid reason, field-error
placement below the input, status-region isolation, computed invalid styling,
localized visible messages, axe after the invalid state is reached, and
Leptos/Dioxus parity over the same state matrix.

#### Procedure

1. **Coverage report.** The command depends on which crate the change touches — this workspace measures host-target and wasm32 paths separately because adapter code (`ars-leptos`, `ars-dioxus`, `ars-dom`, the `ars-i18n` `web-intl` path, the adapter test harnesses) has `target_arch = "wasm32"` / `feature = "web"` code that bare host-target `cargo llvm-cov` cannot reach.

    **For native-only crates** (`ars-components`, `ars-collections`, `ars-core`, `ars-forms`, `ars-interactions`, `ars-a11y`, `xtask`):

    ```bash
    cargo llvm-cov test -p <crate> --features <relevant> --lib --no-fail-fast --text -- <module>::
    ```

    **For adapter crates / `web-intl` paths** (`ars-leptos`, `ars-dioxus`, `ars-dom`, `ars-test-harness-leptos`, `ars-test-harness-dioxus`, or `ars-i18n` when the change is in the `web-intl` feature gate):

    ```bash
    cargo xtask coverage wasm \
      --package <crate> \
      --feature <feat1> --feature <feat2> \
      --file wasm.lcov.info
    ```

    (`--feature` is singular and repeatable, NOT `--features` — it mirrors cargo's `--feature` convention; use `--no-default-features` to drop defaults.) This uses `wasm-bindgen-test`'s experimental coverage recipe with LLVM 22 / `clang-22` (the same recipe CI nightly runs via `cargo xtask ci coverage`). The bare `cargo llvm-cov` command will silently report `0.00%` for the wasm-gated code paths, which is _not_ the same as having no coverage — it means _unmeasured_. Don't trust low numbers on adapter crates from the native path.

    **For a change that touches both native and wasm paths** (e.g., a new ars-i18n helper consumed from both backends, or a refactor in `ars-dom` that affects native callers):

    ```bash
    # Generate the native lcov (workspace, excluding wasm-only crates):
    cargo llvm-cov --workspace \
      --exclude ars-leptos --exclude ars-dioxus \
      --exclude ars-test-harness-leptos --exclude ars-test-harness-dioxus \
      --exclude ars-derive --exclude xtask \
      --lcov --output-path native.lcov.info

    # Generate the wasm lcov for each adapter target you touched:
    cargo xtask coverage wasm --package <wasm-package> --feature <feat> --file wasm.lcov.info

    # Merge without double-counting duplicate lines (each `--file` adds an input):
    cargo xtask coverage merge \
      --file native.lcov.info \
      --file wasm.lcov.info \
      --output merged.lcov.info

    # Enforce per-crate spec-defined thresholds (same gate CI runs):
    cargo xtask coverage check-all --file merged.lcov.info
    ```

    The per-crate thresholds (line% / branch%) are encoded in `xtask::coverage::default_thresholds()` and include the wasm targets. `cargo xtask coverage check-all` is the same gate CI enforces, so a green local run guarantees green CI on the coverage step.

    Read the per-line annotated output from the `--text` invocations. Every `^0` marker is an unhit branch — investigate each one. Categorize as: real test gap / equivalent defensive guard / unreachable dead code. **Expected non-gaps:** no-op callback closures in tests (e.g., `|_| {}`) are intentionally uncovered and not a real gap — CLAUDE.md calls this out explicitly.

    **Not measured by either path:** `ars-derive` (proc-macro, runs in the compiler — exercised indirectly via `crates/ars-core/tests/derive_contract.rs`). Don't expect coverage numbers here.

2. **Mutation testing — NOT a gate here.** Do **not** run `cargo xmutants` as part of this audit, and do not block handoff on a clean local mutation run. Mutation coverage is owned by the Nightly CI `mutation-testing` job; surviving (`MISSED`) mutants it reports are triaged in a dedicated follow-up (add a killing test, or add a justified `.cargo/mutants.toml` `exclude_re` for a true equivalent mutation), not during initial delivery. Lean on the coverage report (step 1) and the breadth audit (step 3) to judge test quality instead. (A voluntary local mutation pass is allowed but never required.)

3. **Test-type breadth audit.** Walk the table above. For each test type, ask: _"Does this kind of change need this kind of test, and do we have one?"_ Examples:
    - New framework-agnostic component with an anatomy table → spec-conformance test required
    - New public function → doc test with `# Examples` required
    - New invariants on a computation pipeline → proptest invariants strongly recommended
    - New `AttrMap` helpers → snapshot tests for each branch required
    - New adapter browser event or reactive DOM path → focused wasm test required
    - New supported reference outcome, visible state, workflow, or cross-adapter parity claim → E2E/browser parity proof required

    For adapter tasks, explicitly split findings into:
    - **wasm gap**: SSR cannot prove the adapter/browser wiring, so add a focused `wasm-bindgen-test`;
    - **E2E gap**: users can see or operate the behavior, or the parity matrix marks it supported, so add an E2E/widget/browser assertion.

    Do not let one bucket satisfy the other by implication.

4. **Cumulative assessment.** Present the before/after metrics in a table:

| Metric                | Before this phase | After this phase |
| --------------------- | ----------------- | ---------------- |
| Lines covered         | x%                | y%               |
| Mutations caught      | x/total           | y/total          |
| Unit tests            | N                 | M                |
| Doc tests             | N                 | M                |
| Spec-conformance test | yes/no            | yes/no           |

#### Output

A list of gaps with concrete reproducers (specific test inputs that trigger the missed branch). For each gap, classify the disposition (real-gap-add-test / equivalent-mutation-skip / dead-code-remove). End with a clear _"land all"_ and the verification commands to confirm.

#### Anti-pattern: chasing 100% line coverage

The goal is **not** "100% line coverage at any cost". The goal is _"strong, behavior-constraining tests with no unreachable defensive code, and every test type that should exist does exist"_. A 99% line-coverage report whose tests make positive-direction assertions is stronger than a 100% report full of assertion-free walks. If the last 1% of uncovered code is a genuinely unreachable defensive guard, leave it. (Mutation testing — the eventual measure of whether tests _constrain_ behavior — is owned by Nightly CI, not this audit.)

---

## Adapter Component Parity Loop

Run this section only when the implementation adds or materially changes a
component in `crates/ars-leptos` or `crates/ars-dioxus`.

Read `docs/implementation/adapter-component-delivery.md`, then read
`docs/implementation/adapter-components/12-parity-audit-loop.md`. Use the
task's sketch under `docs/implementation/sketches/` as the working document.

Complete at least three passes:

1. **Reference outcome pass** — check that every reference behavior observed
   with `playwright-cli` is represented as a matrix row, split by user-visible
   outcome. Add rows for distinct validation messages, reset outcomes,
   localized messages, keyboard paths, focus paths, and live-region outcomes.
   Separate reference outcomes from reference API shapes: React Aria's
   TypeScript/React API is not a shape to copy into Rust, Leptos, or Dioxus.
   Each row must record whether ars-ui reaches the outcome through an
   `IdiomaticEquivalent`, `SameNativeBehavior`, `HigherLevelComposition`, or
   `OutOfScopeApiShape`. Treat API shape as a gap only when the underlying
   outcome cannot be expressed through ars-ui's public contract.
2. **Consumer reality pass** — inspect actual adapter usage, all six widgets,
   fixtures, and demos. Any raw native control, sibling error UI, hardcoded
   component text, or duplicated component policy used to prove a supported row
   is `WidgetOnlyWorkaround` and must be fixed before handoff.
   For adapter primitives, audit every core `Part` enum variant and every
   adapter-rendered structural node. If consumers may need to style or position
   that node, it should be a public stylable part. If it remains private, the
   adapter spec must record why and name the supported styling alternative.
   Low-level primitive roots should follow the Checkbox standard and be named
   `Root` inside the component module; treat semantic root names such as
   `Field`, `Fieldset`, or `Form` in adapter primitive modules as API drift
   unless the spec explicitly documents a higher-level wrapper.
   Required structural nodes with public parts must keep an adapter fallback
   when omitted and must prove that explicit parts suppress the fallback so only
   one semantic node exists. Treat missing part exposure, missing fallback
   suppression, or undocumented private structural nodes as adapter API gaps.
   For Leptos adapters, user-facing semantic text props that can update with
   locale or application state should flow as `TextProp`; the public `t(...)`
   helper returns `Memo<String>` and should be accepted directly through
   `TextProp`. Flag parallel helpers, ad-hoc translation closures in examples,
   or static raw strings for placeholders, accessible labels, validation/status
   text, live announcements, and semantic labels behind custom views. Do not
   require this for DOM tokens and relationships. Verify adapter-owned message
   resolution keeps messages grouped with the locale that selected them via
   `use_messages_and_locale(...)` (Leptos reactive signal, Dioxus render-time
   tuple) instead of resolving messages and reading locale separately.
   such as `id`, `form`, `class`, `style`, `name`, or `aria-labelledby`;
   those remain explicit serialized strings unless the component spec says
   otherwise.
   Also audit example-owned logic directly: widgets and fixtures may own sample
   data, controlled values, callback sinks, consumer-owned copy, routing,
   layout, and styling, but they must not implement component-owned validation,
   ARIA relationships, keyboard/focus behavior, selection, drag/drop, loading,
   layout, popup state, or localized-message policy. Treat any such logic as an
   adapter or agnostic API gap unless the sketch explicitly marks it
   consumer-owned.
3. **I18n/a11y/test proof pass** — attach adapter SSR/unit tests, focused wasm
   tests for adapter/browser wiring, E2E or browser assertions for
   user-visible workflows and cross-adapter parity, locale/direction evidence,
   accessibility evidence, and `playwright-cli` artifact paths to each
   supported row. Do not treat wasm proof as E2E proof unless the row is
   strictly about low-level adapter/browser wiring.

Continue looping until no row remains `Unknown`, `Unverified`, `ContractGap`,
`AdapterApiGap`, or `WidgetOnlyWorkaround`. The only acceptable final row
statuses are `ReferenceOutcomeMatched`, `IntentionallyDifferent`, and
`OutOfScopeWithReason`.

If any gap remains, the implementation is `partial`; surface the rows to the
user instead of claiming outcome parity.

---

## After the audit phases

Run the full verification suite one final time before handing off to the user:

```bash
# 1. Unit + integration tests, both feature-on and feature-off variants
cargo test -p <crate> --features <relevant> --all-targets
cargo test -p <crate> --lib                       # without the optional features

# 2. Workspace warning sweep. This uses the repo clippy wrapper without
#    `-D warnings`, so pending warnings are visible before the user handoff.
cargo xclippy

# 3. Workspace clippy (must be -D warnings clean)
cargo clippy --workspace --all-targets --all-features --exclude ars-i18n -- -D warnings

# 4. Backend matrices for any feature-gated code
cargo clippy -p ars-i18n --no-default-features --features std,icu4x --all-targets -- -D warnings
cargo clippy -p ars-i18n --no-default-features --features std,web-intl --all-targets -- -D warnings

# 5. Docs build (0 new warnings on the changed crate)
cargo doc -p <crate> --no-deps --all-features

# 6. Spec validation + snippet check
cargo xtask spec validate
cargo test -p xtask --test spec_corpus_compile_snippets

# 7. Snapshot orphan check
cargo insta test --workspace --features <relevant> --unreferenced=reject

# 8. (Mutation testing is NOT run here — it is Nightly-CI-driven.)

# 9. Coverage thresholds — uses the same gate CI enforces. The recipe
#    depends on which crate the change touches: native-only changes can
#    use `cargo llvm-cov ... --lcov`; changes that touch adapter code or
#    `web-intl` paths require `cargo xtask coverage wasm` + `merge` first
#    (see Phase 3 above). Then:
cargo xtask coverage check-all --file lcov.info

# 10. Adapter builds reachable with feature wiring
cargo build -p ars-leptos --features icu4x
cargo build -p ars-dioxus --features icu4x,web
```

Treat any `cargo xclippy` warning in changed code as audit fallout: fix the root
cause, or use `#[expect(..., reason = "...")]` only when the suppression is
deliberate and documented. Do not hand off with known warnings in the changed
surface.

Then write **one consolidated user summary** covering all three phases' findings + the verification table. Hand off for user review per CLAUDE.md workflow step 8 (which was the old step 7 before this skill was inserted).

After the user approves commit/push and the PR is opened, **read and follow**
`.agents/skills/waiting-for-codex-review/SKILL.md` through to Codex 👍. That
skill is AGENTS.md step 12 — posting `@codex review` is only the trigger inside
the loop, not a substitute for the full poll/triage/fix cycle.

## Anti-patterns

- **Deferring** any finding ("we can fix that in a follow-up"). This is the rule the user set explicitly. Breaking it makes the audit ceremonial.
- **Loading the user with options mid-phase.** Make a clear best-outcome recommendation. Only escalate to `AskUserQuestion` when there's a genuine judgment call the user must make (e.g., behavior choices with real tradeoffs, not implementation details).
- **Skipping rounds of Phase 2** because the first round was clean. The second round nearly always finds something the first ignored.
- **Treating coverage % as the goal.** The goal is behavior-constraining tests and breadth-complete coverage, not a number.
- **Inventing cosmetic edits** the audits didn't surface. If a phase doesn't find a real finding, don't fabricate one — say "this round found nothing" and proceed.
- **Re-running tests after every micro-edit** during a phase. Land a batch of related findings, then verify, then move on.
- **Forgetting to update the named-tests file** when a new test exposes a real bug fixed in the same phase. The fix and the test land together.

## What this skill is not

- A code-review skill. Code review (style, naming, simplification) belongs to `pr-review-toolkit:code-reviewer` and runs after the PR is opened.
- A pre-implementation skill. Before implementing, `feature-dev:feature-dev` and `superpowers:writing-plans` handle planning.
- A way to revisit acceptance criteria. The acceptance criteria are fixed by the GitHub issue. This skill assumes they're already met.

## Worked example — task #204

The flow this skill encodes was validated on GitHub issue #204 (`task: Implement Highlight agnostic core`). The initial implementation passed all 8 named acceptance-criterion tests on first try. Then:

- **Phase 1** surfaced 16 drift findings, including a silent contract violation (the spec said _"Unicode case folding"_ but the impl used `to_lowercase` — they aren't equivalent: under `to_lowercase`, query `"ss"` does not match text `"ß"`, violating the spec's explicit eszett contract). Fix: add `ars_i18n::case_fold`, switch the matching pipeline, rewrite the German test to assert the actual three-direction equivalence.
- **Phase 2 round 1** surfaced 4 more items (adapter spec alignment, adapter feature wiring, roundtrip invariant test, proptest invariants).
- **Phase 2 round 2** surfaced 3 more (a `case_fold` vs `to_lowercase` semantic gap, foundation/10 parametric-anatomy concept, `cargo xmutants` scout).
- **Phase 3** surfaced 4 coverage gaps (dead `continue` branch, `merge_ranges` else branch, fold-expansion partial-match defense, spec-conformance test). Pushed line coverage from 98.67% to 99.91% with all 57 mutations caught.

The cumulative effect: PR landed with one user review pass instead of three, and the spec contract (`ß ↔ ss`) is actually true instead of cosmetically true. That's the value this skill creates.
