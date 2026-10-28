use std::collections::HashMap;
use std::sync::OnceLock;

const LOCALES: &[(&str, &str)] = &[
    (
        "en",
        include_str!("../../locales/en.lproj/Localizable.strings"),
    ),
    (
        "ru",
        include_str!("../../locales/ru.lproj/Localizable.strings"),
    ),
    (
        "uk",
        include_str!("../../locales/uk.lproj/Localizable.strings"),
    ),
    (
        "de",
        include_str!("../../locales/de.lproj/Localizable.strings"),
    ),
    (
        "fr",
        include_str!("../../locales/fr.lproj/Localizable.strings"),
    ),
    (
        "es",
        include_str!("../../locales/es.lproj/Localizable.strings"),
    ),
    (
        "it",
        include_str!("../../locales/it.lproj/Localizable.strings"),
    ),
    (
        "pt-BR",
        include_str!("../../locales/pt-BR.lproj/Localizable.strings"),
    ),
    (
        "pl",
        include_str!("../../locales/pl.lproj/Localizable.strings"),
    ),
    (
        "tr",
        include_str!("../../locales/tr.lproj/Localizable.strings"),
    ),
    (
        "ja",
        include_str!("../../locales/ja.lproj/Localizable.strings"),
    ),
    (
        "ko",
        include_str!("../../locales/ko.lproj/Localizable.strings"),
    ),
    (
        "zh-Hans",
        include_str!("../../locales/zh-Hans.lproj/Localizable.strings"),
    ),
];

struct Catalog {
    language: &'static str,
    strings: HashMap<String, String>,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(|| {
        let language = sys_locale::get_locales()
            .find_map(|locale| match_language(&locale))
            .unwrap_or("en");
        let source = LOCALES
            .iter()
            .find(|(code, _)| *code == language)
            .map_or("", |(_, text)| *text);
        Catalog {
            language,
            strings: parse_strings(source),
        }
    })
}

fn match_language(locale: &str) -> Option<&'static str> {
    let locale = locale.split('.').next()?.replace('_', "-");
    let lower = locale.to_lowercase();
    if lower.starts_with("zh") {
        return Some("zh-Hans");
    }
    if lower.starts_with("pt") {
        return Some("pt-BR");
    }
    let language = lower.split('-').next()?;
    LOCALES
        .iter()
        .map(|(code, _)| *code)
        .find(|code| *code == language)
}

pub fn language() -> &'static str {
    catalog().language
}

pub fn tr(key: &str) -> String {
    catalog()
        .strings
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

pub fn trf(key: &str, args: &[&str]) -> String {
    let mut text = tr(key);
    for (index, arg) in args.iter().enumerate() {
        text = text.replace(&format!("%{}$@", index + 1), arg);
    }
    for arg in args {
        if let Some(position) = text.find("%@") {
            text.replace_range(position..position + 2, arg);
        }
    }
    text
}

fn parse_strings(source: &str) -> HashMap<String, String> {
    source
        .lines()
        .filter_map(|line| {
            let mut parts = quoted_parts(line);
            Some((parts.next()?, parts.next()?))
        })
        .collect()
}

fn quoted_parts(line: &str) -> impl Iterator<Item = String> + '_ {
    let mut chars = line.chars();
    std::iter::from_fn(move || {
        chars.by_ref().find(|c| *c == '"')?;
        let mut value = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => value.push(chars.next()?),
                '"' => return Some(value),
                _ => value.push(c),
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_escaped_strings() {
        let parsed = parse_strings(r#""Say \"hi\"" = "Скажи \"привет\"";"#);
        assert_eq!(parsed["Say \"hi\""], "Скажи \"привет\"");
    }

    #[test]
    fn matches_regional_locales() {
        assert_eq!(match_language("ru_RU.UTF-8"), Some("ru"));
        assert_eq!(match_language("zh-CN"), Some("zh-Hans"));
        assert_eq!(match_language("pt-PT"), Some("pt-BR"));
        assert_eq!(match_language("xx-YY"), None);
    }
}
