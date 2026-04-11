//! Shared workspace helpers for `ars-i18n` feature-matrix splitting.

use std::{fs, io, path::Path};

use toml::Value as TomlValue;

/// The two mutually exclusive `ars-i18n` backend features.
const I18N_BACKENDS: [&str; 2] = ["icu4x", "web-intl"];

/// Read `crates/ars-i18n/Cargo.toml` and build one feature list per backend.
///
/// Each returned feature list contains every declared feature except `default`
/// and the other backend, so callers can run `ars-i18n` once with the `icu4x`
/// backend and once with the `web-intl` backend without hand-maintaining two
/// separate feature strings.
pub(crate) fn i18n_feature_lists() -> io::Result<[String; 2]> {
    let path = Path::new("crates/ars-i18n/Cargo.toml");
    let content = fs::read_to_string(path)?;
    parse_i18n_feature_lists(&content, path)
}

fn parse_i18n_feature_lists(content: &str, path: &Path) -> io::Result<[String; 2]> {
    let doc = content.parse::<toml::Table>().map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("{path:?}: {error}"))
    })?;

    let features = doc
        .get("features")
        .and_then(TomlValue::as_table)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{path:?}: missing [features] table"),
            )
        })?;

    let common = features
        .keys()
        .map(String::as_str)
        .filter(|feature| *feature != "default" && !I18N_BACKENDS.contains(feature))
        .collect::<Vec<_>>();

    Ok(I18N_BACKENDS.map(|backend| {
        let mut all = common.clone();
        all.push(backend);
        all.join(",")
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_i18n_feature_lists_splits_mutually_exclusive_backends() {
        let content = r#"
[features]
default = ["std", "gregorian", "icu4x"]
std = []
gregorian = []
buddhist = []
icu4x = []
web-intl = []
"#;

        let [icu4x, web_intl] =
            parse_i18n_feature_lists(content, Path::new("crates/ars-i18n/Cargo.toml"))
                .expect("feature parsing should succeed");

        assert_eq!(icu4x, "buddhist,gregorian,std,icu4x");
        assert_eq!(web_intl, "buddhist,gregorian,std,web-intl");
    }

    #[test]
    fn parse_i18n_feature_lists_requires_features_table() {
        let error = parse_i18n_feature_lists("", Path::new("crates/ars-i18n/Cargo.toml"))
            .expect_err("missing features table should fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("missing [features] table"));
    }
}
