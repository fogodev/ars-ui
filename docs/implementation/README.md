# Implementation Program

This directory turns the specification into an executable delivery program.

The repo is expected to operate with these rules:

- Build the platform first: workspace, core contracts, subsystems, harnesses, then components.
- Run TDD-style delivery: define the exact tests first, then implement the minimum code needed.
- Keep the specification synchronized with implementation. If implementation changes the intended contract, update the relevant spec in the same task.
- Use GitHub Projects with issue-backed items only.
- Keep most agent-ready tasks at `1`, `2`, `3`, or `5` points. `8` is exceptional. `13` must be split before pickup.

Start with:

1. [roadmap.md](./roadmap.md) for the phase plan.
2. [project-board.md](./project-board.md) for fields, workflow, and sizing rules.
3. [initial-backlog.md](./initial-backlog.md) for the seed epics and first implementation tasks.
4. The GitHub Project and initial issue backlog are expected to be kept in sync with these docs.
5. [adapter-contract.md](./adapter-contract.md) for adapter work obligations and spec-sync checklist.
6. [foundation-gap-audit.md](./foundation-gap-audit.md) for the backlog reset that defers `#24` and promotes the missing foundation contracts into issue-ready follow-on tasks.
7. [foundation-completion-roadmap.md](./foundation-completion-roadmap.md) for the remaining foundation work (interactions, collections, DOM positioning, i18n, and browser `web-intl` follow-ons) organized into five delivery waves before component work begins.
