# Widgets Examples Ownership

The widgets examples are split so adapter-level component work can be parallelized by spec category without agents editing the same files.

For category implementation work, edit only:

- the target adapter crate files under `crates/ars-leptos` or `crates/ars-dioxus`;
- the matching example category module, such as `examples/widgets-leptos/src/categories/utility.rs`;
- styling files for that example variant only when the category demo requires visual styling changes.

`main.rs`, `text.rs`, and `categories/mod.rs` are coordination files. Do not edit them during normal category implementation work unless adding, removing, or renaming a top-level spec category.

Translated category copy belongs in the category module's local text enum, not in the root `WidgetsText` enum. For example, utility demo copy belongs in `UtilityText`, navigation demo copy belongs in `NavigationText`, and empty input placeholder copy belongs in `InputText`.

Do not introduce a shared widgets example crate unless the project explicitly decides to trade merge isolation for deduplication.
