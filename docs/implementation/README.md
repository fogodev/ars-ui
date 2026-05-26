# Implementation Program

This directory turns the specification into an executable delivery program.

The repo is expected to operate with these rules:

- Build the platform first: workspace, core contracts, subsystems, harnesses, then components.
- Run TDD-style delivery: define the exact tests first, then implement the minimum code needed.
- Keep the specification synchronized with implementation. If implementation changes the intended contract, update the relevant spec in the same task.
- Port implementation improvements back into the spec in the same PR — not as follow-up cleanup. See [Spec synchronization](#spec-synchronization) below.
- For every new or materially changed framework-agnostic component, add spec-conformance tests for its anatomy/public contract and run a targeted mutation test for the component source file (`cargo xmutants -p ars-components -f crates/ars-components/src/<category>/<component>/mod.rs` or the equivalent file path). Prefer `cargo xmutants` over bare `cargo mutants` — it applies the same per-crate `--features` profile as Nightly CI (for example `ars-components/i18n`) so snapshot and i18n-gated tests match CI. Triage every `MISSED` mutant in the same task: add tests for real gaps, or document true equivalence in `.cargo/mutants.toml`. This is a pre-PR implementation/audit gate; after the PR is opened, agents **MUST NOT** rerun mutation tests during Codex review rounds unless the user explicitly asks for another mutation run.
- Use GitHub Projects with issue-backed items only.
- Keep most agent-ready tasks at `1`, `2`, `3`, or `5` points. `8` is exceptional. `13` must be split before pickup.

## Spec synchronization

Implementation may refine the contract during delivery, but the spec remains the
authoritative artifact. Every task that adds or materially changes implementation
code must **port improvements back into the spec in the same PR**.

This is not limited to intentional contract changes. Also update the spec when
implementation reveals the existing spec is incomplete or stale:

- **Missing public surface** — fluent builders, helper methods (for example
  `Props::popover_props()`), enum token helpers (for example `Variant::as_str()`).
- **Incomplete code examples** — prose or accessibility sections promise behavior
  that the §1 API sketch omits (for example `dir` resolution on content attrs).
- **Dependency specs** — composition layers need accessors on shared machines
  that downstream specs already call but the dependency spec never documented
  (for example `popover::Api::content_id()`, `heading_id()`, `toggle()`,
  `close()` added for ContextualHelp).
- **Anatomy and ARIA tables** — §2/§3 tables missing attributes the connect API
  already emits (`aria-controls`, `data-ars-variant`, `data-ars-state`, `dir`,
  and similar).
- **Default-source clarity** — implementation relies on struct-update defaults
  instead of redundant explicit fields; spec should describe the effective
  contract, not only the verbose spelling.

Use the post-implementation audit Phase 1 drift pass (`.agents/skills/post-implementation-audit/SKILL.md`) to surface these. When the implementation is correct and the spec is incomplete, update the spec — treat that polish as in-scope delivery work, not a nice-to-have follow-up.

After spec edits, run:

```bash
cargo xtask spec validate
```

For adapter work, also follow the spec-sync checklist in
[adapter-contract.md](./adapter-contract.md).

## PR closeout and Codex review

After the user approves commit and the PR branch is pushed, **read and follow**
`.agents/skills/waiting-for-codex-review/SKILL.md` in full. This is mandatory
for every PR — the same requirement as AGENTS.md Development Workflow step 12.

Posting `@codex review` is only step 2 of that skill. It is **not** a substitute
for the skill. Agents must stay in the skill's poll loop until Codex leaves 👍:

1. Post `@codex review` (once per review pass).
2. Poll PR reactions and unresolved Codex review threads on the cadence in the
   skill.
3. If 👀 drops and threads appear, address every finding in the same PR with
   the same rigor as `post-implementation-audit`, then reply inline, resolve
   threads, push, and post `@codex review` again.
4. Repeat until 👍 is present. Green CI alone is not the merge gate.

Do not hand the PR back to the user as "done" after push until step 4 completes
or the skill's escalation rules say to ping the user (for example Codex never
starts after 5 minutes).

Start with:

1. [roadmap.md](./roadmap.md) for the phase plan.
2. [project-board.md](./project-board.md) for fields, workflow, and sizing rules.
3. [initial-backlog.md](./initial-backlog.md) for the seed epics and first implementation tasks.
4. The GitHub Project and initial issue backlog are expected to be kept in sync with these docs.
5. [adapter-contract.md](./adapter-contract.md) for adapter work obligations and spec-sync checklist.
6. [foundation-gap-audit.md](./foundation-gap-audit.md) for the backlog reset that defers `#24` and promotes the missing foundation contracts into issue-ready follow-on tasks.
7. [foundation-completion-roadmap.md](./foundation-completion-roadmap.md) for the remaining foundation work (interactions, collections, DOM positioning, i18n, and browser `web-intl` follow-ons) organized into five delivery waves before component work begins.
