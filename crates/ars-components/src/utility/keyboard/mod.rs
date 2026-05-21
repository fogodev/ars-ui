//! `Keyboard` component machine and connect API.
//!
//! `Keyboard` is a stateless attribute mapper that renders shortcut text
//! inside a semantic `<kbd>` element with optional platform-aware modifier
//! mapping (`Mod` → `⌘` on macOS, `Ctrl`/`Strg`/etc. on others) and
//! locale-aware modifier names. It has no state machine — the
//! framework-agnostic core consists solely of [`Props`], [`Part`], and
//! [`Api`].
//!
//! Platform detection (`Props::is_mac`) is supplied by the adapter via
//! `PlatformEffects::is_mac_platform()` (see
//! `spec/foundation/01-architecture.md` §2.2.7); during SSR the adapter
//! defaults the flag to `false` (non-Mac) and re-renders after hydration.
//!
//! See `spec/components/utility/keyboard.md` for the authoritative
//! contract.

use alloc::{string::String, vec::Vec};

use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr};
use ars_i18n::Locale;

/// Props for the `Keyboard` component.
#[derive(Clone, Debug, Default, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// The keyboard shortcut text to display (e.g., `"⌘C"`, `"Ctrl+Shift+P"`).
    /// When [`Props::platform_aware`] is `true`, modifier tokens such as
    /// `"Mod"`, `"Shift"`, `"Alt"`, `"Ctrl"`, and `"Meta"` are
    /// auto-mapped to the platform-appropriate symbol or label.
    pub shortcut: String,

    /// When `true`, automatically maps generic modifier tokens to
    /// platform-specific symbols: `"Mod"` → `"⌘"` (macOS) / `"Ctrl"`
    /// (others), `"Alt"` → `"⌥"` (macOS) / `"Alt"`, `"Shift"` → `"⇧"`
    /// (macOS) / `"Shift"`, `"Meta"` → `"⌘"` (macOS) / `"Win"` (others).
    /// Default behaviour (`false`) renders the shortcut text as-is.
    pub platform_aware: bool,

    /// Whether the current platform is macOS. The adapter provides this
    /// value via `PlatformEffects::is_mac_platform()` (see
    /// `spec/foundation/01-architecture.md` §2.2.7). During SSR, defaults
    /// to `false` (non-Mac); client-side hydration updates the display if
    /// the actual platform differs.
    pub is_mac: bool,

    /// When `true`, the rendered `<kbd>` is purely decorative and the
    /// agnostic core emits `aria-hidden="true"` on the root so screen
    /// readers skip it. Use when the shortcut is a visual reminder only
    /// and the action it triggers is announced through some other path
    /// (e.g., a button label). Default: `false` — assistive tech announces
    /// the shortcut text. Inside Menu items the parent `Shortcut` part
    /// already manages `aria-hidden` on its own wrapper, so leave this
    /// `false` there to avoid duplication.
    pub decorative: bool,
}

impl Props {
    /// Returns fresh [`Props`] with the documented defaults — equivalent
    /// to [`Default::default`], offered as the entry point for the
    /// builder chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the shortcut text rendered inside the `<kbd>` element.
    ///
    /// When [`Props::platform_aware`] is `true`, modifier tokens
    /// (`"Mod"`, `"Shift"`, `"Alt"`, `"Ctrl"`, `"Meta"`) inside the
    /// string are auto-mapped to the platform-appropriate symbol or
    /// label.
    #[must_use]
    pub fn shortcut(mut self, value: impl Into<String>) -> Self {
        self.shortcut = value.into();
        self
    }

    /// Enables or disables platform-aware modifier mapping.
    #[must_use]
    pub const fn platform_aware(mut self, value: bool) -> Self {
        self.platform_aware = value;
        self
    }

    /// Sets the macOS-platform flag.
    ///
    /// Adapter-supplied; tests pass it directly. See
    /// `spec/foundation/01-architecture.md` §2.2.7.
    #[must_use]
    pub const fn is_mac(mut self, value: bool) -> Self {
        self.is_mac = value;
        self
    }

    /// Marks the rendered `<kbd>` as decorative — the agnostic core
    /// will emit `aria-hidden="true"` on the root. See
    /// [`Props::decorative`] for usage guidance.
    #[must_use]
    pub const fn decorative(mut self, value: bool) -> Self {
        self.decorative = value;
        self
    }
}

/// DOM parts of the `Keyboard` component.
#[derive(ComponentPart)]
#[scope = "keyboard"]
pub enum Part {
    /// The root `<kbd>` element. Contains the formatted shortcut text.
    Root,
}

/// The API for the `Keyboard` component.
///
/// Owns its [`Props`] (matching the stateless-Api convention shared with
/// `separator`, `visually_hidden`, `landmark`, etc.) and is queried via
/// [`Api::root_attrs`] and [`Api::display_text`], or through the
/// [`ConnectApi`] dispatch.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
}

impl Api {
    /// Creates a new `Api` instance owning the given props.
    #[must_use]
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Returns a reference to the underlying [`Props`].
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the component's instance ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.props.id
    }

    /// Returns the raw shortcut string as it was provided in props,
    /// unaffected by platform-aware mapping.
    #[must_use]
    pub fn shortcut(&self) -> &str {
        &self.props.shortcut
    }

    /// Returns whether platform-aware modifier mapping is enabled.
    #[must_use]
    pub const fn platform_aware(&self) -> bool {
        self.props.platform_aware
    }

    /// Returns whether the props were constructed for the macOS platform.
    #[must_use]
    pub const fn is_mac(&self) -> bool {
        self.props.is_mac
    }

    /// Returns whether the root `<kbd>` is marked as decorative
    /// (hidden from the accessibility tree via `aria-hidden`).
    #[must_use]
    pub const fn decorative(&self) -> bool {
        self.props.decorative
    }

    /// Returns the attributes for the root `<kbd>` element.
    ///
    /// Always emits `data-ars-scope="keyboard"` and
    /// `data-ars-part="root"`. When [`Props::decorative`] is `true`,
    /// also emits `aria-hidden="true"` so the shortcut is skipped by
    /// assistive technology — used when the action is already announced
    /// elsewhere (e.g., via the parent button's label) and the `<kbd>`
    /// is a visual reminder only. Inside Menu items the parent
    /// `Shortcut` part owns the aria-hidden — keep [`Props::decorative`]
    /// `false` there to avoid duplication.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.props.decorative {
            attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }

        attrs
    }

    /// Returns the formatted shortcut text, applying platform-aware
    /// modifier mapping when [`Props::platform_aware`] is `true` and
    /// passing through `shortcut` verbatim otherwise.
    ///
    /// When `locale` is `Some`, modifier names are localised per spec
    /// §3.2 (`Ctrl` → `Strg` in `de`, `Shift` → `Maj` in `fr`, `Shift`
    /// → `Mayús` in `es`, `Shift` → `Maiusc` in `it`, …). Locales not
    /// covered by the table fall back to English labels.
    #[must_use]
    pub fn display_text(&self, locale: Option<&Locale>) -> String {
        if self.props.platform_aware {
            format_platform_shortcut(&self.props.shortcut, self.props.is_mac, locale)
        } else {
            self.props.shortcut.clone()
        }
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

/// Modifier keys recognised by [`format_platform_shortcut`].
///
/// Strongly typed so a typo in the formatter or in the localisation
/// table is a compile error rather than a silent fall-through to the
/// "unknown token" branch.
#[derive(Clone, Copy)]
enum Modifier {
    /// The platform modifier — `⌘` on macOS, `Ctrl` (or its localised
    /// equivalent) on every other platform.
    Mod,

    /// The shift key — `⇧` on macOS, `Shift` (or its localised
    /// equivalent) on every other platform.
    Shift,

    /// The alt / option key — `⌥` on macOS, `Alt` on every other
    /// platform.
    Alt,

    /// The literal control key — `⌃` on macOS, `Ctrl` (localised) on
    /// every other platform.
    Ctrl,

    /// The OS / Windows / Super key — `⌘` on macOS, `Win` on every
    /// other platform.
    Meta,
}

impl Modifier {
    /// Parses one segment of a `+`-separated shortcut into a
    /// [`Modifier`], or returns `None` for unknown tokens (printable
    /// keys, function keys, etc.).
    fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "Mod" => Self::Mod,
            "Shift" => Self::Shift,
            "Alt" => Self::Alt,
            "Ctrl" => Self::Ctrl,
            "Meta" => Self::Meta,
            _ => return None,
        })
    }

    /// Returns the macOS symbol for this modifier.
    const fn mac_symbol(self) -> &'static str {
        match self {
            Self::Mod | Self::Meta => "⌘",
            Self::Shift => "⇧",
            Self::Alt => "⌥",
            Self::Ctrl => "⌃",
        }
    }

    /// Returns the default English label for this modifier on
    /// non-macOS platforms.
    const fn english_label(self) -> &'static str {
        match self {
            // `Mod` expands to `Ctrl` on non-mac platforms.
            Self::Mod | Self::Ctrl => "Ctrl",
            Self::Shift => "Shift",
            Self::Alt => "Alt",
            Self::Meta => "Win",
        }
    }
}

/// Returns the localised display name for a modifier key on non-macOS
/// platforms.
///
/// Implements the modifier-localisation table from spec §3.2 — only the
/// entries that diverge from English on physical keyboards are
/// translated; all others fall back to [`Modifier::english_label`].
fn localized_modifier(modifier: Modifier, locale: Option<&Locale>) -> &'static str {
    let lang = locale.map_or("en", Locale::language);

    match (modifier, lang) {
        // German localises `Ctrl` (and `Mod`, which expands to Ctrl) to
        // `Strg` and `Shift` to `Umschalt`.
        (Modifier::Mod | Modifier::Ctrl, "de") => "Strg",

        (Modifier::Shift, "de") => "Umschalt",

        // French, Spanish, and Italian localise only `Shift`.
        (Modifier::Shift, "fr") => "Maj",

        (Modifier::Shift, "es") => "Mayús",

        (Modifier::Shift, "it") => "Maiusc",

        // Every other (modifier, language) pair falls back to the
        // English label — including Japanese (which uses English labels
        // on physical keyboards) and any locale not in the §3.2 table.
        _ => modifier.english_label(),
    }
}

/// Maps generic modifier tokens to platform-specific symbols.
///
/// `is_mac` is supplied by the caller (via the adapter's
/// `PlatformEffects::is_mac_platform()` query). On macOS, modifier
/// symbols are concatenated with no separator (`"⌘C"`); on every other
/// platform tokens are joined with `"+"` (`"Ctrl+C"`).
fn format_platform_shortcut(shortcut: &str, is_mac: bool, locale: Option<&Locale>) -> String {
    shortcut
        .split('+')
        .map(|part| match (Modifier::parse(part), is_mac) {
            (Some(modifier), true) => modifier.mac_symbol(),
            (Some(modifier), false) => localized_modifier(modifier, locale),
            (None, _) => part,
        })
        .collect::<Vec<_>>()
        .join(if is_mac { "" } else { "+" })
}

#[cfg(test)]
mod tests {
    use ars_core::HasId;
    use ars_i18n::Locale;
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn props(shortcut: &str, platform_aware: bool, is_mac: bool) -> Props {
        Props {
            shortcut: String::from(shortcut),
            platform_aware,
            is_mac,
            ..Props::default()
        }
    }

    fn parse(tag: &str) -> Locale {
        Locale::parse(tag).expect("locale tag should parse")
    }

    // ── Props ──────────────────────────────────────────────────────

    #[test]
    fn props_default_values() {
        let p = Props::default();

        assert_eq!(p.id, "");
        assert_eq!(p.shortcut, "");
        assert!(!p.platform_aware);
        assert!(!p.is_mac);
        assert!(!p.decorative);
    }

    #[test]
    fn props_builder_round_trips() {
        // `Props::new()` returns the documented defaults and the chained
        // setters mutate exactly the matching fields, leaving the others
        // at their default values.
        let p = Props::new()
            .id("kbd-build")
            .shortcut("Mod+C")
            .platform_aware(true)
            .is_mac(true)
            .decorative(true);

        assert_eq!(p.id, "kbd-build");
        assert_eq!(p.shortcut, "Mod+C");
        assert!(p.platform_aware);
        assert!(p.is_mac);
        assert!(p.decorative);

        // `Props::new()` is equivalent to `Default::default()`.
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_setters_are_idempotent_per_field() {
        // Each setter overrides only its own field; later calls overwrite
        // earlier ones for the same field.
        let p = Props::new()
            .platform_aware(true)
            .platform_aware(false)
            .shortcut("Mod+A")
            .shortcut("Mod+B");

        assert!(!p.platform_aware);
        assert_eq!(p.shortcut, "Mod+B");
    }

    #[test]
    fn props_clone_and_partial_eq_round_trip() {
        let original = Props {
            id: String::from("kbd-clone"),
            shortcut: String::from("Mod+S"),
            platform_aware: true,
            is_mac: false,
            decorative: true,
        };

        let cloned = original.clone();

        assert_eq!(cloned, original);

        let mutated = Props {
            is_mac: true,
            ..original.clone()
        };

        assert_ne!(mutated, original);
    }

    #[test]
    fn props_has_id_derive_round_trips() {
        // Exercises the methods the `HasId` derive emits directly on
        // `Props` (the `Api::id()` accessor only goes through one of them).
        let mut p = props("Mod", false, false).with_id(String::from("kbd-a"));

        assert_eq!(HasId::id(&p), "kbd-a");

        p.set_id(String::from("kbd-b"));

        assert_eq!(HasId::id(&p), "kbd-b");
    }

    #[test]
    fn props_and_api_are_send_sync() {
        // The agnostic core's public types must be `Send + Sync` so
        // adapters and ahead-of-time computation paths can shuttle
        // values between threads (web handlers, async server functions,
        // etc. — see workspace `feedback_messagefn_send_sync.md`). This
        // assertion fails to compile if a future refactor introduces a
        // non-thread-safe field (e.g., `Rc`, `Cell`).
        const fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Props>();
        assert_send_sync::<Api>();
        assert_send_sync::<Part>();
    }

    #[test]
    fn api_clone_round_trips() {
        // The `Clone` derive on `Api` must produce a value structurally
        // equal to the source. This locks the property against a future
        // refactor adding a non-clone-coherent field (instance counter,
        // allocation ID, RNG state, etc.).
        let original = Api::new(Props {
            id: String::from("kbd-clone"),
            shortcut: String::from("Mod+Shift+Q"),
            platform_aware: true,
            is_mac: true,
            decorative: true,
        });

        let cloned = original.clone();

        assert_eq!(cloned.props(), original.props());
        assert_eq!(cloned.root_attrs(), original.root_attrs());
        assert_eq!(
            cloned.display_text(None),
            original.display_text(None),
            "cloned Api must produce identical display_text",
        );
    }

    #[test]
    fn props_and_api_debug_impl_is_non_empty() {
        // Smoke test guarding against an accidental empty `impl Debug`.
        let p = props("Mod+C", true, true);
        let api = Api::new(p.clone());

        let props_dbg = format!("{:?}", api.props());

        let api_dbg = format!("{api:?}");

        assert!(props_dbg.contains("Props"), "Props Debug = {props_dbg}");
        assert!(api_dbg.contains("Api"), "Api Debug = {api_dbg}");
    }

    // ── Api accessors ──────────────────────────────────────────────

    #[test]
    fn api_exposes_props_fields() {
        // Cover the `true` branch of every bool accessor with one
        // Props, and the `false` branch with a default-constructed
        // Props. Both branches are required: mutation testing surfaced
        // that asserting only the `false` branch silently permits a
        // regression where the accessor always returns `false`.
        let truthy = Props {
            id: String::from("kbd-7"),
            shortcut: String::from("Mod+Shift+P"),
            platform_aware: true,
            is_mac: true,
            decorative: true,
        };

        let api = Api::new(truthy.clone());

        assert_eq!(api.id(), "kbd-7");
        assert_eq!(api.shortcut(), "Mod+Shift+P");
        assert!(api.platform_aware());
        assert!(api.is_mac());
        assert!(api.decorative());
        assert_eq!(api.props(), &truthy);

        let falsy = props("", false, false);
        let falsy_api = Api::new(falsy.clone());

        assert_eq!(falsy_api.id(), "");
        assert_eq!(falsy_api.shortcut(), "");
        assert!(!falsy_api.platform_aware());
        assert!(!falsy_api.is_mac());
        assert!(!falsy_api.decorative());
    }

    // ── Connect / API ──────────────────────────────────────────────

    #[test]
    fn part_attrs_dispatches_root() {
        let p = props("Mod+C", true, true);

        let api = Api::new(p.clone());

        let attrs = api.part_attrs(Part::Root);

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("keyboard"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn api_root_attrs_equals_part_attrs_root_across_branches() {
        // The `ConnectApi` dispatch must produce exactly what the
        // inherent `root_attrs` method produces, across every output-
        // affecting prop combination.
        let cases = [
            props("Mod", false, false),
            props("Mod+C", true, true),
            props("Meta", true, false),
            props("Mod+Shift+P", true, true),
            // decorative path:
            Props {
                decorative: true,
                ..props("Mod+C", true, true)
            },
        ];

        for p in cases {
            let api = Api::new(p.clone());

            assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        }
    }

    // ── root_attrs branches ────────────────────────────────────────

    #[test]
    fn root_attrs_emit_scope_and_part() {
        // Spec §1.2 / §2: agnostic core always emits
        // `data-ars-scope="keyboard"` and `data-ars-part="root"`.
        let p = props("Mod+C", true, true);

        let attrs = Api::new(p.clone()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("keyboard"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn root_attrs_omit_aria_hidden_by_default() {
        // Spec §3.1: when `decorative` is `false` (default), screen
        // readers announce the shortcut text naturally — the agnostic
        // core MUST NOT emit `aria-hidden`. Inside Menu items the
        // parent `Shortcut` part owns aria-hidden on its wrapper, so
        // the `<kbd>` itself must remain announced (the wrapper is the
        // hidden node).
        let p = props("Mod+C", true, true);

        let attrs = Api::new(p.clone()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), None);
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));
    }

    #[test]
    fn root_attrs_emit_aria_hidden_when_decorative() {
        // Spec §3.1: `decorative = true` ⇒ emit `aria-hidden="true"`
        // on the `<kbd>` so assistive tech skips the purely-visual
        // shortcut hint.
        let p = Props {
            decorative: true,
            ..props("Mod+C", true, true)
        };

        let attrs = Api::new(p.clone()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }

    #[test]
    fn decorative_and_non_decorative_branches_produce_different_attrs() {
        // Defensive cross-branch inequality: flipping `decorative`
        // MUST change the AttrMap, otherwise the boolean attribute
        // accidentally became unconditional or got dropped.
        let semantic = Api::new(props("Mod+C", true, true)).root_attrs();

        let decorative = Api::new(Props {
            decorative: true,
            ..props("Mod+C", true, true)
        })
        .root_attrs();

        assert_ne!(semantic, decorative);
    }

    #[test]
    fn root_attrs_are_invariant_under_shortcut_and_platform() {
        // Holding `decorative` constant, varying every other prop must
        // produce an identical `AttrMap`.
        let baseline = Api::new(props("Mod+C", true, true)).root_attrs();

        for p in [
            props("Mod+Shift+P", true, false),
            props("Mod+C", false, false),
            props("Meta", true, false),
            props("", false, false),
        ] {
            assert_eq!(Api::new(p.clone()).root_attrs(), baseline);
        }
    }

    // ── display_text branches ──────────────────────────────────────

    #[test]
    fn display_text_returns_shortcut_verbatim_when_platform_aware_false() {
        // Covers the `else` branch: `platform_aware = false` ⇒ no
        // mapping, no localisation; the shortcut string is returned
        // exactly as it was passed in.
        let p = props("Mod+C", false, true);

        assert_eq!(Api::new(p.clone()).display_text(None), "Mod+C");
        assert_eq!(
            Api::new(p.clone()).display_text(Some(&parse("de"))),
            "Mod+C"
        );
    }

    #[test]
    fn display_text_maps_mod_to_command_on_mac() {
        // Issue test 4: `Mod+C` with `is_mac=true` ⇒ `⌘C`, no separator.
        let p = props("Mod+C", true, true);

        assert_eq!(Api::new(p.clone()).display_text(None), "⌘C");
    }

    #[test]
    fn display_text_maps_meta_to_win_on_windows() {
        // Issue test 5: `Meta` with `is_mac=false` ⇒ `Win`.
        let p = props("Meta", true, false);

        assert_eq!(Api::new(p.clone()).display_text(None), "Win");
    }

    #[test]
    fn display_text_formats_human_readable_shortcut() {
        // Issue test 6 (non-Mac): `Mod+Shift+P` ⇒ `Ctrl+Shift+P`.
        let p = props("Mod+Shift+P", true, false);

        assert_eq!(Api::new(p.clone()).display_text(None), "Ctrl+Shift+P");
    }

    #[test]
    fn display_text_maps_full_modifier_set_on_mac() {
        // Mac modifier symbols across the full set the spec maps.
        let cases = [
            ("Mod+C", "⌘C"),
            ("Shift+Tab", "⇧Tab"),
            ("Alt+Enter", "⌥Enter"),
            ("Ctrl+K", "⌃K"),
            ("Meta+Q", "⌘Q"),
        ];

        for (shortcut, expected) in cases {
            let p = props(shortcut, true, true);

            assert_eq!(Api::new(p.clone()).display_text(None), expected);
        }
    }

    #[test]
    fn display_text_maps_full_modifier_set_off_mac_default_locale() {
        // Non-Mac modifier names without a locale fall back to English.
        let cases = [
            ("Mod+C", "Ctrl+C"),
            ("Shift+Tab", "Shift+Tab"),
            ("Alt+Enter", "Alt+Enter"),
            ("Ctrl+K", "Ctrl+K"),
            ("Meta+Q", "Win+Q"),
        ];

        for (shortcut, expected) in cases {
            let p = props(shortcut, true, false);

            assert_eq!(Api::new(p.clone()).display_text(None), expected);
        }
    }

    #[test]
    fn display_text_localizes_ctrl_to_strg_in_de() {
        // Spec §3.2: German localises `Ctrl` ⇒ `Strg` (and `Mod`,
        // which expands to Ctrl on non-Mac) and `Shift` ⇒ `Umschalt`.
        let de = parse("de");

        let mod_c = props("Mod+C", true, false);
        let ctrl_c = props("Ctrl+C", true, false);
        let shift_p = props("Shift+P", true, false);

        assert_eq!(Api::new(mod_c.clone()).display_text(Some(&de)), "Strg+C");
        assert_eq!(Api::new(ctrl_c.clone()).display_text(Some(&de)), "Strg+C");
        assert_eq!(
            Api::new(shift_p.clone()).display_text(Some(&de)),
            "Umschalt+P"
        );
    }

    #[test]
    fn display_text_localizes_shift_to_maj_in_fr() {
        // Spec §3.2: French localises `Shift` ⇒ `Maj`; `Ctrl` and
        // `Alt` remain English.
        let fr = parse("fr");

        let shift_p = props("Shift+P", true, false);
        let ctrl_c = props("Ctrl+C", true, false);

        assert_eq!(Api::new(shift_p.clone()).display_text(Some(&fr)), "Maj+P");
        assert_eq!(Api::new(ctrl_c.clone()).display_text(Some(&fr)), "Ctrl+C");
    }

    #[test]
    fn display_text_localizes_shift_to_mayus_in_es() {
        // Spec §3.2: Spanish localises `Shift` ⇒ `Mayús`.
        let es = parse("es");

        let shift_p = props("Shift+P", true, false);

        assert_eq!(Api::new(shift_p.clone()).display_text(Some(&es)), "Mayús+P");
    }

    #[test]
    fn display_text_localizes_shift_to_maiusc_in_it() {
        // Spec §3.2: Italian localises `Shift` ⇒ `Maiusc`.
        let it = parse("it");

        let shift_p = props("Shift+P", true, false);

        assert_eq!(
            Api::new(shift_p.clone()).display_text(Some(&it)),
            "Maiusc+P"
        );
    }

    #[test]
    fn display_text_uses_english_when_locale_none() {
        // `locale = None` ⇒ `lang` defaults to `"en"` ⇒ no localisation
        // table entry matches ⇒ the English label is returned.
        let p = props("Mod+Shift+Alt+P", true, false);

        assert_eq!(Api::new(p.clone()).display_text(None), "Ctrl+Shift+Alt+P");
    }

    #[test]
    fn display_text_uses_english_for_unlocalised_languages() {
        // Spec §3.2: Japanese (`ja`) uses English modifier labels on
        // physical keyboards; languages outside the spec table also
        // fall back to English. This locks the behaviour so an
        // accidental wildcard branch cannot regress it.
        let ja = parse("ja");
        let pt = parse("pt-BR");

        let mod_shift = props("Mod+Shift+P", true, false);

        assert_eq!(
            Api::new(mod_shift.clone()).display_text(Some(&ja)),
            "Ctrl+Shift+P"
        );
        assert_eq!(
            Api::new(mod_shift.clone()).display_text(Some(&pt)),
            "Ctrl+Shift+P"
        );
    }

    #[test]
    fn display_text_handles_single_token() {
        // Input with no `+` separator must still be mapped correctly
        // — the splitter yields a single segment and the join is a
        // no-op.
        let mac_mod = props("Mod", true, true);
        let win_meta = props("Meta", true, false);

        assert_eq!(Api::new(mac_mod.clone()).display_text(None), "⌘");
        assert_eq!(Api::new(win_meta.clone()).display_text(None), "Win");
    }

    #[test]
    fn display_text_does_not_localize_alt_in_any_language() {
        // Spec §3.2: `Alt` is `"Alt"` in every locale on the table —
        // German, French, Spanish, Italian, Japanese all use the
        // English label. This test catches an accidental
        // `(Modifier::Alt, _) => "..."` entry that would diverge from
        // the table.
        for tag in ["de", "fr", "es", "it", "ja"] {
            let p = props("Alt+Enter", true, false);

            assert_eq!(
                Api::new(p.clone()).display_text(Some(&parse(tag))),
                "Alt+Enter",
                "Alt should not be localised in `{tag}`",
            );
        }
    }

    #[test]
    fn display_text_does_not_localize_ctrl_outside_german() {
        // Spec §3.2: only German (`de`) translates `Ctrl` (and `Mod`,
        // which expands to Ctrl). French, Spanish, Italian, and
        // Japanese keep the English label. Catches an accidental
        // `(Modifier::Ctrl, "fr") => "..."` entry.
        for tag in ["fr", "es", "it", "ja"] {
            let p = props("Ctrl+C", true, false);

            assert_eq!(
                Api::new(p.clone()).display_text(Some(&parse(tag))),
                "Ctrl+C",
                "Ctrl should not be localised in `{tag}`",
            );
        }
    }

    #[test]
    fn display_text_handles_empty_and_malformed_input() {
        // Defensive: empty / leading-`+` / trailing-`+` / consecutive-`+`
        // shortcuts must not panic and must produce stable output.
        // `split('+')` on these inputs yields empty segments which fall
        // through `Modifier::parse` to the "unknown token" branch,
        // emitting `""`. The join then drops them on macOS (concat with
        // no separator) and preserves them on non-Mac (joined with `+`).
        let mac = |s: &str| Api::new(props(s, true, true)).display_text(None);
        let non_mac = |s: &str| Api::new(props(s, true, false)).display_text(None);

        assert_eq!(mac(""), "");
        assert_eq!(non_mac(""), "");

        // Trailing `+` keeps a trailing separator on non-Mac.
        assert_eq!(mac("Mod+"), "⌘");
        assert_eq!(non_mac("Mod+"), "Ctrl+");

        // Leading `+` keeps a leading separator on non-Mac.
        assert_eq!(mac("+Mod"), "⌘");
        assert_eq!(non_mac("+Mod"), "+Ctrl");

        // Consecutive `+` produce empty segments; the formatter is
        // tolerant rather than rejecting.
        assert_eq!(mac("Mod++C"), "⌘C");
        assert_eq!(non_mac("Mod++C"), "Ctrl++C");
    }

    #[test]
    fn display_text_passes_unknown_tokens_through() {
        // Tokens that are not in the modifier set are emitted verbatim
        // on both platforms.
        let mac = props("Mod+ArrowLeft", true, true);
        let non_mac = props("Mod+ArrowLeft", true, false);

        assert_eq!(Api::new(mac.clone()).display_text(None), "⌘ArrowLeft");
        assert_eq!(
            Api::new(non_mac.clone()).display_text(None),
            "Ctrl+ArrowLeft"
        );
    }

    // ── Cross-branch inequality ────────────────────────────────────

    #[test]
    fn mac_and_non_mac_produce_different_text_for_same_shortcut() {
        // Defensive: the platform mapping MUST diverge between macOS
        // and non-Mac for tokens in the modifier set, otherwise the
        // platform-aware path collapses into a no-op.
        let mac = props("Mod+C", true, true);
        let non_mac = props("Mod+C", true, false);

        assert_ne!(
            Api::new(mac.clone()).display_text(None),
            Api::new(non_mac.clone()).display_text(None)
        );
    }

    #[test]
    fn localized_and_default_produce_different_text() {
        // Defensive: the locale parameter MUST be consulted for the
        // languages in the table.
        let p = props("Shift+P", true, false);

        assert_ne!(
            Api::new(p.clone()).display_text(None),
            Api::new(p.clone()).display_text(Some(&parse("de")))
        );
    }

    #[test]
    fn display_text_ignores_locale_on_mac() {
        // Spec §3.2: "macOS uses universal symbols (⌘ ⇧ ⌥ ⌃) regardless
        // of language." Locks the invariant: when `is_mac == true`, the
        // `locale` argument has no effect on `display_text`. Catches a
        // regression where someone wires `locale` into the Mac path.
        let p = props("Mod+Shift+Alt+Ctrl+P", true, true);
        let baseline = Api::new(p.clone()).display_text(None);

        for tag in ["de", "fr", "es", "it", "ja", "pt-BR"] {
            assert_eq!(
                Api::new(p.clone()).display_text(Some(&parse(tag))),
                baseline,
                "macOS output must be locale-invariant; diverged for `{tag}`",
            );
        }
    }

    #[test]
    fn platform_aware_off_disables_localization() {
        // Defensive: `platform_aware = false` ⇒ the locale parameter
        // is ignored entirely.
        let p = props("Mod+C", false, false);

        assert_eq!(
            Api::new(p.clone()).display_text(Some(&parse("de"))),
            Api::new(p.clone()).display_text(None)
        );
    }

    // ── Snapshots ──────────────────────────────────────────────────

    #[test]
    fn keyboard_root_attrs_snapshot() {
        // `root_attrs` is invariant under every prop EXCEPT
        // `decorative`, so the non-decorative path needs one snapshot.
        let p = props("Mod+C", true, true);

        assert_snapshot!(
            "keyboard_root_attrs",
            snapshot_attrs(&Api::new(p.clone()).root_attrs())
        );
    }

    #[test]
    fn keyboard_root_attrs_decorative_snapshot() {
        // Locks the decorative branch's emission of
        // `aria-hidden="true"`.
        let p = Props {
            decorative: true,
            ..props("Mod+C", true, true)
        };

        assert_snapshot!(
            "keyboard_root_attrs_decorative",
            snapshot_attrs(&Api::new(p.clone()).root_attrs())
        );
    }

    #[test]
    fn keyboard_display_mod_c_mac_snapshot() {
        let p = props("Mod+C", true, true);

        assert_snapshot!(
            "keyboard_display_mod_c_mac",
            Api::new(p.clone()).display_text(None)
        );
    }

    #[test]
    fn keyboard_display_mod_c_non_mac_snapshot() {
        let p = props("Mod+C", true, false);

        assert_snapshot!(
            "keyboard_display_mod_c_non_mac",
            Api::new(p.clone()).display_text(None)
        );
    }

    #[test]
    fn keyboard_display_mod_c_de_snapshot() {
        let p = props("Mod+C", true, false);

        assert_snapshot!(
            "keyboard_display_mod_c_de",
            Api::new(p.clone()).display_text(Some(&parse("de")))
        );
    }

    #[test]
    fn keyboard_display_shift_p_fr_snapshot() {
        let p = props("Shift+P", true, false);

        assert_snapshot!(
            "keyboard_display_shift_p_fr",
            Api::new(p.clone()).display_text(Some(&parse("fr")))
        );
    }

    #[test]
    fn keyboard_display_shift_p_es_snapshot() {
        let p = props("Shift+P", true, false);

        assert_snapshot!(
            "keyboard_display_shift_p_es",
            Api::new(p.clone()).display_text(Some(&parse("es")))
        );
    }

    #[test]
    fn keyboard_display_shift_p_it_snapshot() {
        let p = props("Shift+P", true, false);

        assert_snapshot!(
            "keyboard_display_shift_p_it",
            Api::new(p.clone()).display_text(Some(&parse("it")))
        );
    }

    #[test]
    fn keyboard_display_verbatim_when_disabled_snapshot() {
        let p = props("Mod+C", false, true);

        assert_snapshot!(
            "keyboard_display_verbatim_when_disabled",
            Api::new(p.clone()).display_text(None)
        );
    }

    #[test]
    fn keyboard_display_meta_non_mac_snapshot() {
        let p = props("Meta", true, false);

        assert_snapshot!(
            "keyboard_display_meta_non_mac",
            Api::new(p.clone()).display_text(None)
        );
    }
}
