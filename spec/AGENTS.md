# Spec Directory Guide

## File Conventions

- `_category.md` files contain category overviews and shared conventions
- Component files use kebab-case matching the component name
- Shared types in `shared/` are referenced by component frontmatter
- Headings are numbered (`## 1. State Machine`, `### 1.1 States`)

## Spec Changes During Implementation

The repo is in active implementation. Spec edits inside `spec/` should usually be tied to a concrete GitHub implementation task or PR, not made as detached cleanup.

- Read the implementation issue before changing spec text.
- Keep cited spec sections synchronized with the implementation task that discovered the mismatch.
- If an adapter-specific implementation exposes a missing shared abstraction, promote that abstraction into the appropriate foundation/shared spec instead of documenting it only in framework code.
- Do not leave placeholder notes for later reconciliation; resolve the spec/code mismatch in the same task whenever possible.

## Adding a New Component

1. Create `components/{category}/{component-name}.md` with YAML frontmatter
2. Follow the section structure defined in `foundation/10-component-spec-template.md` for the appropriate tier
3. Add an entry to `manifest.toml` with path, category, deps, and related
4. Update `foundation/02-component-catalog.md` with the new component
5. Run `cargo run -p spec-tool -- validate` to verify consistency

## Component File Format

```markdown
---
component: ComponentName
category: { category }
tier: stateless | stateful | complex
foundation_deps: [architecture, accessibility, ...]
shared_deps: [date-time-types, ...]
related: [sibling-component, ...]
---

# ComponentName
```

See `foundation/10-component-spec-template.md` for the canonical section structure, ordering rules, conformance checklist, and skeleton examples for each tier.
