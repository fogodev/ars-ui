---
document: adapter-component-spec-template
type: foundation
---

# Adapter Component Specification Template

This document defines the canonical authoring rules for adapter component specs under adapter trees such as:

- `spec/leptos-components/*`
- `spec/dioxus-components/*`

It also defines the required structure for adapter category documents such as:

- `spec/leptos-components/{category}/_category.md`
- `spec/dioxus-components/{category}/_category.md`

The purpose of this template is to make adapter specs:

- implementation-facing
- reviewable
- auditable against the framework-agnostic specs
- consistent across categories

The framework-agnostic component spec remains the behavior source of truth. The adapter spec is the implementation-facing source of truth for framework-specific API shape, composition, lifecycle wiring, semantic repair, platform fallback, and verification guidance.

## 1. Scope and Relationship to Core Component Specs

Adapter component specs do not redefine the core component contract. They map it onto a concrete framework adapter.

Every adapter spec must:

- reference the corresponding framework-agnostic component spec
- preserve the core behavior, state machine, anatomy, and accessibility contract unless an intentional adapter deviation is documented
- define the framework-facing API and rendering strategy
- define adapter-owned behavior explicitly instead of leaving it implied

Adapter specs must never assume that an implementer will infer adapter behavior from the agnostic spec alone.

## 2. Mandatory Promotion Rule for Adapter-Owned Behavior

If the framework-agnostic component spec says or implies that something is:

- handled by the adapter
- a framework adapter concern
- a platform concern
- a DOM or runtime concern
- a composition concern
- an SSR or hydration concern
- a ref or node ownership concern
- a cleanup concern
- a semantic repair concern
- a form-bridging concern
- a fallback concern

then the adapter spec must restate that behavior explicitly in adapter-facing sections.

This includes both:

- hard requirements
- recommendation-level adapter guidance

### 2.1 Hard Requirements

Adapter-owned behavior that is required for correctness or parity must appear in the relevant implementation-facing sections, and usually also in:

- `Adapter Invariants`
- `Test Scenarios`
- `Test Oracle Notes`
- `Implementation Checklist`

### 2.2 Recommendation-Level Guidance

Adapter-facing guidance that remains a recommendation rather than a hard invariant must still be restated explicitly.

Recommendation-level guidance usually belongs in:

- `Framework-Specific Behavior`
- `Accessibility and SSR Notes`
- `Implementation Checklist`
- `Parity Summary and Intentional Deviations`

Do not silently upgrade a recommendation into a hard invariant unless correctness or parity requires it.

## 3. Adapter-Owned Concern Categories

When auditing or authoring adapter specs, treat these as the standard concern categories:

- native semantic repair
- event normalization
- handler composition
- node or ref ownership
- SSR or hydration gating
- client-only side effects
- platform-specific fallback
- form bridging
- cleanup or disposal ordering
- accessibility repair
- async reconciliation
- slot or `as_child` composition
- registration or descendant propagation
- platform capability lookup
- diagnostics and warning policy

If a concern exists in the agnostic spec, the adapter spec must place it in the section where an implementer would look first.

## 4. YAML Frontmatter for Adapter Component Specs

Every adapter component file begins with YAML frontmatter:

```yaml
---
adapter: leptos | dioxus
component: ComponentName
category: { category }
source: components/{category}/{component-kebab}.md
source_foundation: foundation/08-adapter-leptos.md | foundation/09-adapter-dioxus.md
---
```

Required fields:

- `adapter`
- `component`
- `category`
- `source`
- `source_foundation`

The `source` field must point to the corresponding framework-agnostic component spec.

## 5. Canonical Adapter Component Section Structure

Every adapter component spec must use this top-level section order:

1. `Purpose and Adapter Scope`
2. `Public Adapter API`
3. `Mapping to Core Component Contract`
4. `Part Mapping`
5. `Attr Merge and Ownership Rules`
6. `Composition / Context Contract`
7. `Prop Sync and Event Mapping`
8. `Registration and Cleanup Contract`
9. `Ref and Node Contract`
10. `State Machine Boundary Rules`
11. `Callback Payload Contract`
12. `Failure and Degradation Rules`
13. `Identity and Key Policy`
14. `SSR and Client Boundary Rules`
15. `Performance Constraints`
16. `Implementation Dependencies`
17. `Recommended Implementation Sequence`
18. `Anti-Patterns`
19. `Consumer Expectations and Guarantees`
20. `Platform Support Matrix`
21. `Debug Diagnostics and Production Policy`
22. `Shared Adapter Helper Notes`
23. `Framework-Specific Behavior`
24. `Canonical Implementation Sketch`
25. `Reference Implementation Skeleton`
26. `Adapter Invariants`
27. `Accessibility and SSR Notes`
28. `Parity Summary and Intentional Deviations`
29. `Test Scenarios`
30. `Test Oracle Notes`
31. `Implementation Checklist`

Section numbering is mandatory and sequential with no gaps.

## 6. Section Content Requirements

### 6.1 `Purpose and Adapter Scope`

Must state:

- which agnostic component is being mapped
- which framework and version the spec targets
- what the adapter spec adds beyond the agnostic spec

### 6.2 `Public Adapter API`

Must define the framework-facing component signature, slot shape, prop conventions, and callback surface.

Examples:

- Leptos `#[component]` function signature with `Children`, slot props, and `Signal<T>` usage
- Dioxus `#[component]` function signature with `Element`, props (with the `#[derive(Props)]` macro), and signal usage

### 6.3 `Mapping to Core Component Contract`

Must summarize:

- props parity
- part parity
- known adapter deviations

This is the first place to mention whether the adapter is full parity, partial parity, or parity with explicit deviations.

### 6.4 `Part Mapping`

Must define:

- each core part or structural node
- required vs optional status
- rendered target
- ownership
- attr source
- notes when multiplicity or indirection matters

### 6.5 `Attr Merge and Ownership Rules`

Must state:

- which attrs come from the core API
- which attrs are adapter-owned
- which attrs are consumer-owned
- merge order
- semantic repair requirements

If the agnostic spec says the adapter must add `role`, `aria-*`, or host repair attrs, that rule must be explicit here.

### 6.6 `Composition / Context Contract`

Must state:

- provided contexts
- required consumed contexts
- optional consumed contexts
- failure behavior for missing required context
- composition rules for polymorphic or compound structures

For compound component APIs, this section must also state the adapter naming scheme:

- the root component uses the bare component name inside its module namespace
- child parts drop the redundant component prefix and rely on module scoping
- the primary child-part context is named `Context`
- secondary helper or provider contexts use descriptive non-prefixed names
- missing-context diagnostics use module-qualified part names in panic/expect text

### 6.7 `Prop Sync and Event Mapping`

Must define:

- controlled vs uncontrolled sync
- post-mount sync rules
- event normalization
- handler ordering
- form bridging
- async reconciliation paths

If the agnostic spec mentions adapter event repair, callback order, or browser-native interaction differences, this section must make that concrete.

### 6.8 `Registration and Cleanup Contract`

Must define:

- what registers
- when registration happens
- what identity key is used
- when cleanup occurs
- exact cleanup action

This is mandatory for:

- repeated items
- descendants
- temporary resources
- listeners
- observers
- blob URLs
- timers
- form bridges

### 6.9 `Ref and Node Contract`

Must use a table with these columns:

- `Target part / node`
- `Ref required?`
- `Ref owner`
- `Node availability`
- `Composition rule`
- `Notes`

Use these exact node-availability labels:

- `server-safe absent`
- `client-only`
- `required after mount`
- `always structural, handle optional`

### 6.10 `State Machine Boundary Rules`

Must explicitly separate:

- machine-owned state
- adapter-local derived bookkeeping
- forbidden local mirrors
- allowed snapshot-read contexts

If the agnostic spec describes optimistic update rollback, async reconciliation, or machine-owned controlled state, this section must restate it explicitly.

### 6.11 `Callback Payload Contract`

Must use a table with these columns:

- `Callback`
- `Payload source`
- `Payload shape`
- `Timing`
- `Cancelable?`
- `Notes`

Use these exact payload-source labels:

- `raw framework event`
- `normalized adapter payload`
- `machine-derived snapshot`
- `none`

### 6.12 `Failure and Degradation Rules`

Must define failure or degradation behavior for:

- missing required context
- duplicates
- unsupported platform behavior
- impossible prop combinations
- invalid child count
- missing refs or late nodes
- SSR-only absence of browser APIs

Use these exact policy labels:

- `fail fast`
- `degrade gracefully`
- `warn and ignore`
- `no-op`

### 6.13 `Identity and Key Policy`

Must define identity rules for:

- repeated items
- registrations
- hidden inputs
- server-error keys
- temporary resources when relevant

Use these exact identity-source labels:

- `data-derived`
- `instance-derived`
- `composite`
- `not applicable`

### 6.14 `SSR and Client Boundary Rules`

Must make explicit:

- what renders on the server
- what waits until hydration or mount
- what refs are unavailable until mount
- what listeners, timers, or effects are client-only
- what structure must remain hydration-stable

### 6.15 `Performance Constraints`

Must contain concrete rules, not generic advice.

Typical requirements include:

- avoid listener churn
- avoid rebuilding registries
- avoid replacing structural nodes unnecessarily
- keep cleanup instance-scoped
- keep measurement and observer work incremental

### 6.16 `Implementation Dependencies`

Must define:

- dependencies on utilities or helpers that should exist first
- whether each dependency is required or recommended
- dependency type
- why the dependency matters

Use these exact dependency-type labels:

- `conceptual`
- `shared helper`
- `context contract`
- `composition contract`
- `behavioral prerequisite`

### 6.17 `Recommended Implementation Sequence`

Must be a numbered list of implementation steps.

It is component-local build order, not roadmap planning.

### 6.18 `Anti-Patterns`

Every bullet must begin with `Do not`.

Anti-patterns must be specific to the component’s likely implementation failures.

### 6.19 `Consumer Expectations and Guarantees`

Must use short bullets beginning with:

- `Consumers may assume ...`
- `Consumers must not assume ...`

### 6.20 `Platform Support Matrix`

Leptos specs must use:

`Capability / behavior | Browser client | SSR | Notes`

Dioxus specs must use:

`Capability / behavior | Web | Desktop | Mobile | SSR | Notes`

Use these exact support labels:

- `full support`
- `fallback path`
- `client-only`
- `SSR-safe empty`
- `not applicable`

### 6.21 `Debug Diagnostics and Production Policy`

Must use a table with:

- `Condition`
- `Debug build behavior`
- `Production behavior`
- `Notes`

Use these exact behavior labels where applicable:

- `debug warning`
- `fail fast`
- `warn and ignore`
- `degrade gracefully`
- `no-op`

### 6.22 `Shared Adapter Helper Notes`

Must use a table with:

- `Helper concept`
- `Required?`
- `Responsibility`
- `Reused by`
- `Notes`

These notes describe reusable implementation infrastructure, not public runtime API.

### 6.23 `Framework-Specific Behavior`

Only true adapter divergence belongs here.

Shared adapter obligations should not be hidden in this section.

### 6.24 `Canonical Implementation Sketch`

This is illustrative only.

It must not contradict any contract section.

### 6.25 `Reference Implementation Skeleton`

Required for every stateful or behavior-heavy utility, and recommended for other stateful adapter categories too.

This skeleton must:

- be tighter than the canonical sketch
- show helper boundaries
- show sequencing
- show ownership and cleanup boundaries
- show SSR or client gating where relevant

For simple semantic components, this section may explicitly say that no expanded skeleton beyond the canonical sketch is required.

### 6.26 `Adapter Invariants`

This is the mandatory landing zone for non-negotiable adapter rules.

High-risk adapter-owned behavior should usually appear here as well as in:

- `Implementation Checklist`
- `Test Oracle Notes`

### 6.27 `Accessibility and SSR Notes`

Must cover any final accessibility or hydration-sensitive guidance not already captured elsewhere.

### 6.28 `Parity Summary and Intentional Deviations`

Must include:

- a parity summary
- intentional deviations
- traceability notes for high-risk adapter-owned concerns when helpful

A traceability note is strongly recommended for the hardest components in a category.

### 6.29 `Test Scenarios`

Must enumerate the meaningful scenarios, not just generic headings.

Every scenario added here must correspond to at least one preferred oracle in `Test Oracle Notes`.

### 6.30 `Test Oracle Notes`

Must state the authoritative assertion surface for each important behavior.

Use these exact oracle types where relevant:

- `DOM attrs`
- `machine state`
- `callback order`
- `context registration`
- `rendered structure`
- `hydration structure`
- `cleanup side effects`

For high-risk components, a short cheap verification recipe is recommended.

### 6.31 `Implementation Checklist`

Must use `- [ ]` items.

It is a completion gate, not a prose summary.

## 7. Required Category Document Structure

Every adapter category must include a `_category.md` file.

### 7.1 Category Frontmatter

```yaml
---
adapter: leptos | dioxus
category: { category }
source_foundation: foundation/08-adapter-leptos.md | foundation/09-adapter-dioxus.md
---
```

### 7.2 Required Category Sections

Every `_category.md` must include:

- `# {Category} Components — {Adapter} Adapter`
- `## Scope`
- `## Conventions`
- `## Utility Index` or category-equivalent index

The exact index heading may remain category-specific if the category already uses a different stable naming convention, but the file must still contain a component index.

### 7.3 Required Category Content

The category file must define:

- what the category maps from the agnostic specs
- the adapter and framework version it targets
- broad conventions for props, slots, context, machines, and framework APIs
- the final component section structure used by specs in that tree
- legends for platform matrices and diagnostics tables
- shared terminology
- helper taxonomy
- cross-category consumption notes where useful
- an authoring lint checklist

### 7.4 Required Category-Level Guidance

Every category doc must explicitly state:

- canonical examples are illustrative only
- adapter implementation sections are normative
- adapter-owned behavior must not remain only in the agnostic spec
- recommendation-level adapter guidance must also be restated explicitly
- high-risk adapter-only behavior should land in `Adapter Invariants`, `Implementation Checklist`, and `Test Oracle Notes`
- `Framework-Specific Behavior` is reserved for true framework divergence
- per-component specs remain the primary source of truth when category docs and component docs overlap

## 8. Adapter Audit Requirements

When writing or revising an adapter category, authors must perform a one-to-one audit against the corresponding agnostic specs.

For each agnostic component spec, identify every adapter-defined concern and ensure it is represented in the adapter spec.

At minimum, search for and evaluate phrases such as:

- `adapter must`
- `adapter should`
- `adapter layer`
- `framework adapter`
- `handled in the adapter`
- `platform note`
- `cleanup in adapter`

Phrase matching alone is not sufficient. Authors must also inspect the agnostic prose for implied adapter-owned concerns, especially around:

- DOM semantics
- event composition
- node ownership
- hydration
- browser-only behavior
- context publication
- temporary resource cleanup
- form participation
- diagnostics

## 9. Verification Requirements for Adapter Specs

Adapter authoring is not complete until all of the following are true:

1. the component spec uses the canonical section order
2. every adapter-owned concern from the agnostic spec is restated explicitly
3. every new scenario has an oracle
4. every fallback has a production behavior policy
5. every meaningful platform variance has a matrix row
6. every high-risk component includes clear invariants and checklist coverage
7. framework API examples match the actual supported adapter version

## 10. Adapter Authoring Lint Checklist

- [ ] The agnostic component spec was audited for every adapter-owned requirement and recommendation.
- [ ] No adapter-defined caveat remains only in the agnostic component spec.
- [ ] Shared adapter obligations are documented outside `Framework-Specific Behavior`.
- [ ] All required labels for policy, identity, payload source, support, and diagnostics use the standardized vocabulary.
- [ ] Every new `Test Scenario` has at least one preferred oracle.
- [ ] Every stateful or behavior-heavy component has an implementation skeleton that is tighter than the canonical sketch.
- [ ] The category `_category.md` file states the section structure, legends, helper taxonomy, and authoring rules for the category.

## 11. Expected Outcome

When this template is followed, adapter categories become:

- easier to implement
- easier to review
- easier to audit against the agnostic specs
- easier to keep aligned across Leptos and Dioxus

Most importantly, future implementers no longer need to infer adapter behavior from scattered agnostic caveats. The adapter spec becomes the explicit implementation contract.
