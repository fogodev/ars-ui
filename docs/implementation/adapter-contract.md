# Adapter Contract Reference

This document defines the shared obligations for any work touching the adapter
layer. It consolidates rules from the specification, workflow templates, and
implementation program into a single contributor-facing reference.

## When This Applies

Any PR or issue where:

- Layer = **Adapter**
- Framework = **Leptos**, **Dioxus**, or **Both**
- The change touches `crates/ars-leptos/`, `crates/ars-dioxus/`, or adapter
  specs under `spec/leptos-components/` or `spec/dioxus-components/`

## Obligations

### 1. Foundation Spec References

Adapter work must cite the foundation spec sections it depends on. At minimum:

- `spec/foundation/01-architecture.md` section 6 (Adapter Architecture)
- `spec/foundation/08-adapter-leptos.md` or `spec/foundation/09-adapter-dioxus.md`
- The framework-agnostic component spec for any component being adapted

Issues must list these in the **Spec refs** field. PRs must list them in the
**Spec refs** section.

### 2. Mandatory Promotion Rule

If implementation reveals behavior that is shared across both adapters, that
behavior must be promoted into the appropriate `spec/foundation/` or
`spec/shared/` file. It must not live only in adapter code or adapter specs.

This rule comes from
`spec/foundation/12-adapter-component-spec-template.md` section 2. It applies
to both hard requirements and recommendation-level guidance.

### 3. Deviation Documentation

Any deviation from the framework-agnostic spec must be documented with explicit
justification. Deviations belong in the adapter component spec's
**Parity Summary and Intentional Deviations** section. Undocumented deviations
are review blockers.

### 4. Parity Testing

When both adapters exist for a component, parity testing is required: both
adapters must produce equivalent behavior, ARIA output, and `data-ars-*`
attributes for the same inputs.

If a PR touches only one adapter, state why single-adapter scope is justified
(for example, the other adapter does not yet exist for this component).

### 5. Adapter Concern Checklist

When reviewing or authoring adapter work, verify these concern categories
against the spec (from `12-adapter-component-spec-template.md` section 3):

- [ ] AttrMap merge and ownership
- [ ] Event normalization and handler composition
- [ ] SSR and hydration gating
- [ ] Controlled value synchronization
- [ ] Cleanup and disposal ordering
- [ ] Ref and node ownership
- [ ] Context registration and descendant propagation
- [ ] Native semantic repair
- [ ] Platform-specific fallback
- [ ] Form bridging

Not every concern applies to every PR. Check the ones relevant to the change
and note "N/A" for the rest.

### 6. Adapter Component Spec Structure

New adapter component specs must follow the 31-section structure defined in
`spec/foundation/12-adapter-component-spec-template.md` section 5. Section
numbering is mandatory and sequential with no gaps.

## Spec Synchronization Rules

These rules apply to all implementation work, not just adapter tasks. They are
restated here for completeness.

1. Each task must declare **Spec impact** in the issue.
2. If implementation proves the spec wrong or incomplete, update the spec in the
   same task.
3. Shared abstraction changes go into `spec/foundation/` or `spec/shared/`.
4. Adapter-specific realization belongs in `spec/foundation/08-adapter-leptos.md`,
   `spec/foundation/09-adapter-dioxus.md`, and the per-component adapter specs.
5. Adapter code must not become the only authoritative explanation for behavior
   that future framework ports would need to reproduce.

## Related Documents

- `spec/foundation/01-architecture.md` section 6 — Adapter Architecture
- `spec/foundation/08-adapter-leptos.md` — Leptos adapter contract
- `spec/foundation/09-adapter-dioxus.md` — Dioxus adapter contract
- `spec/foundation/12-adapter-component-spec-template.md` — Adapter component spec authoring rules
- `spec/testing/05-adapter-harness.md` — Adapter parity testing
- `docs/implementation/roadmap.md` — Spec synchronization rules
