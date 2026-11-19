//! The app's language: English (the source texts), Italian or Spanish. The
//! same system as the Knurl app (DooperDrew/knurl `crates/xmc-app/src/i18n.rs`).
//!
//! - Texts written in Rust go through [`t!`](crate::t): `t!("Open Knurl")`,
//!   or with named values, `t!("Could not delete \"{name}\": {e}", name = n,
//!   e = e)`. The English text is the key; a missing translation shows the
//!   English text, never an empty string.
//! - Their translations are in `lang/<code>/app.po`, read into the exe at
//!   build time. the tests below fail when a text used in the code
//!   has no translation, or a translation loses a `{value}` (tests below).
//! - Texts written in the `.slint` files use `@tr(...)`; their translations
//!   are in `lang/<code>/LC_MESSAGES/knurl-probe-app.po`, bundled by `build.rs`, and
//!   follow the language through `slint::select_bundled_translation`.
//!
//! Plain logic, so it builds and is tested on every platform.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// A language the app has texts for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    It,
    Es,
}

impl Lang {
    /// Every language, in the order of the language choice.
    pub const ALL: [Lang; 3] = [Lang::En, Lang::It, Lang::Es];

    /// The code saved in the settings and used for the catalogue folders.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::It => "it",
            Lang::Es => "es",
        }
    }

    /// The language's own name, as the choice shows it (never translated,
    /// so a person finds their language whatever the current one is).
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::It => "Italiano",
            Lang::Es => "Español",
        }
    }

    /// From a saved code; `None` for anything else.
    pub fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }

    /// From a Windows locale name such as `it-IT` or `es_MX`: its language
    /// part decides, and anything the app has no texts for is English.
    pub fn from_locale(locale: &str) -> Lang {
        let language = locale
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        Lang::from_code(&language).unwrap_or(Lang::En)
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Sets the language of every text produced from now on.
pub fn set(lang: Lang) {
    let index = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    CURRENT.store(index as u8, Ordering::Relaxed);
}

/// The language in use (English until [`set`] is called).
pub fn current() -> Lang {
    Lang::ALL
        .get(usize::from(CURRENT.load(Ordering::Relaxed)))
        .copied()
        .unwrap_or(Lang::En)
}

const IT_PO: &str = include_str!("../lang/it/app.po");
const ES_PO: &str = include_str!("../lang/es/app.po");

/// The catalogue of a language: English text to translation.
fn catalogue(lang: Lang) -> Option<&'static HashMap<String, String>> {
    static IT: OnceLock<HashMap<String, String>> = OnceLock::new();
    static ES: OnceLock<HashMap<String, String>> = OnceLock::new();
    match lang {
        Lang::En => None,
        Lang::It => Some(IT.get_or_init(|| parse_po(IT_PO))),
        Lang::Es => Some(ES.get_or_init(|| parse_po(ES_PO))),
    }
}

/// The language for a choice on the welcome screen: 0 follows Windows
/// (`locale` is Windows' display language, e.g. `it-IT`), 1-3 pick one.
pub fn for_choice(choice: i32, locale: &str) -> Lang {
    usize::try_from(choice - 1)
        .ok()
        .and_then(|i| Lang::ALL.get(i).copied())
        .unwrap_or_else(|| Lang::from_locale(locale))
}

/// The code to give `slint::select_bundled_translation`: English is the
/// source text, so it is the empty selection.
pub fn slint_code(lang: Lang) -> &'static str {
    if lang == Lang::En { "" } else { lang.code() }
}

/// `text` in the current language.
pub fn tr(text: &'static str) -> &'static str {
    tr_in(current(), text)
}

/// `text` in `lang`; the English text when there is no translation.
pub fn tr_in(lang: Lang, text: &'static str) -> &'static str {
    catalogue(lang)
        .and_then(|c| c.get(text))
        .filter(|t| !t.is_empty())
        .map_or(text, String::as_str)
}

/// Puts the values in a text: every `{name}` becomes its value. A name with
/// no value stays as written, so a mistake in a translation is visible.
pub fn fill(template: &str, values: &[(&str, &dyn Display)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let name = &after[..close];
                match values.iter().find(|(n, _)| *n == name) {
                    Some((_, value)) => out.push_str(&value.to_string()),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A text in the current language. Without values it is a `&'static str`;
/// with `name = value` pairs it is a `String` with every `{name}` filled.
#[macro_export]
macro_rules! t {
    ($text:literal) => {
        $crate::i18n::tr($text)
    };
    ($text:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::fill(
            $crate::i18n::tr($text),
            &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),+],
        )
    };
}

/// Reads the `msgid` / `msgstr` pairs of a gettext `.po` file: quoted
/// strings, continued on following lines, with `\"`, `\\`, `\n` and `\t`.
/// Entries with an empty `msgid` (the header) or `msgstr` are left out.
pub fn parse_po(text: &str) -> HashMap<String, String> {
    #[derive(PartialEq)]
    enum Field {
        None,
        Id,
        Str,
    }
    fn unquote(line: &str) -> Option<String> {
        let inner = line.trim().strip_prefix('"')?.strip_suffix('"')?;
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        }
        Some(out)
    }
    let mut map = HashMap::new();
    let (mut id, mut value, mut field) = (String::new(), String::new(), Field::None);
    let mut keep = |id: &mut String, value: &mut String| {
        if !id.is_empty() && !value.is_empty() {
            map.insert(std::mem::take(id), std::mem::take(value));
        }
        id.clear();
        value.clear();
    };
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("msgid ") {
            keep(&mut id, &mut value);
            id = unquote(rest).unwrap_or_default();
            field = Field::Id;
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            value = unquote(rest).unwrap_or_default();
            field = Field::Str;
        } else if line.starts_with('"') {
            let more = unquote(line).unwrap_or_default();
            match field {
                Field::Id => id.push_str(&more),
                Field::Str => value.push_str(&more),
                Field::None => {}
            }
        } else if line.is_empty() || line.starts_with('#') {
            // A blank line or a comment ends nothing: the next msgid does.
        } else {
            field = Field::None;
        }
    }
    keep(&mut id, &mut value);
    map
}

/// The `{name}` placeholders of a text, sorted: a translation must keep the
/// same ones as its English text.
#[cfg(test)]
pub fn placeholders(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                names.push(after[..close].to_string());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_locales() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Lang::from_code("fr"), None);
        assert_eq!(Lang::from_locale("it-IT"), Lang::It);
        assert_eq!(Lang::from_locale("es_MX"), Lang::Es);
        assert_eq!(Lang::from_locale("ES-419"), Lang::Es);
        assert_eq!(Lang::from_locale("en-GB"), Lang::En);
        // No texts for it: English, never a guess.
        assert_eq!(Lang::from_locale("de-DE"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
    }

    #[test]
    fn the_choice_picks_a_language_or_follows_windows() {
        assert_eq!(for_choice(0, "it-IT"), Lang::It);
        assert_eq!(for_choice(0, "de-DE"), Lang::En);
        assert_eq!(for_choice(1, "it-IT"), Lang::En);
        assert_eq!(for_choice(2, "en-US"), Lang::It);
        assert_eq!(for_choice(3, "en-US"), Lang::Es);
        assert_eq!(for_choice(9, "es-ES"), Lang::Es, "unknown: follows Windows");
        assert_eq!(slint_code(Lang::En), "");
        assert_eq!(slint_code(Lang::Es), "es");
    }

    #[test]
    fn values_fill_their_names() {
        let n = 3;
        assert_eq!(
            fill("Delete \"{name}\" ({n})", &[("name", &"Train"), ("n", &n)]),
            "Delete \"Train\" (3)"
        );
        // Another order in a translation, a name used twice.
        assert_eq!(
            fill("{b} prima di {a}, poi {b}", &[("a", &1), ("b", &2)]),
            "2 prima di 1, poi 2"
        );
        // A name without a value, or a brace alone, stays visible.
        assert_eq!(fill("{x} and {", &[("a", &1)]), "{x} and {");
        assert_eq!(fill("plain", &[]), "plain");
    }

    #[test]
    fn po_files_are_read() {
        let po = "# comment\nmsgid \"\"\nmsgstr \"Content-Type: text/plain\\n\"\n\n\
            msgid \"Open Knurl\"\nmsgstr \"Apri Knurl\"\n\n\
            msgid \"Long \"\n\"text \\\"{name}\\\"\"\nmsgstr \"\"\n\"Testo \"\n\"lungo «{name}»\"\n\n\
            msgid \"Not translated\"\nmsgstr \"\"\n";
        let map = parse_po(po);
        assert_eq!(
            map.get("Open Knurl").map(String::as_str),
            Some("Apri Knurl")
        );
        assert_eq!(
            map.get("Long text \"{name}\"").map(String::as_str),
            Some("Testo lungo «{name}»")
        );
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn a_missing_translation_shows_the_english_text() {
        assert_eq!(tr_in(Lang::En, "Try again"), "Try again");
        assert_eq!(tr_in(Lang::It, "no such text 123"), "no such text 123");
    }

    /// The string literal that starts at `text` (after its opening quote),
    /// with Rust's escapes undone, and what follows its closing quote.
    fn literal(text: &str) -> Option<(String, &str)> {
        let mut out = String::new();
        let mut chars = text.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Some((out, &text[i + 1..])),
                '\\' => match chars.next()?.1 {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    // A line continuation: the break and the indent go,
                    // and the rest of the literal follows.
                    '\n' => {
                        let rest = chars.as_str().trim_start();
                        let (more, tail) = literal(rest)?;
                        out.push_str(&more);
                        return Some((out, tail));
                    }
                    other => out.push(other),
                },
                other => out.push(other),
            }
        }
        None
    }

    /// Every literal that follows `marker` (`t!(` or `@tr(`) in `source`,
    /// outside `//` comments and outside the file's test module.
    fn texts_after(source: &str, marker: &str) -> Vec<String> {
        // The test module is the last thing in a file; a `#[cfg(test)]`
        // on a single item earlier on ends nothing.
        let end = source
            .rfind("#[cfg(test)]\nmod tests")
            .unwrap_or(source.len());
        let code: String = source[..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut found = Vec::new();
        let mut rest = code.as_str();
        while let Some(at) = rest.find(marker) {
            let after = rest[at + marker.len()..].trim_start();
            // `format!(` ends in `t!(` too: the marker must start a word.
            let inside_a_word = rest[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
            if inside_a_word {
                rest = after;
                continue;
            }
            match after.strip_prefix('"').and_then(literal) {
                Some((text, tail)) => {
                    found.push(text);
                    rest = tail;
                }
                None => rest = after,
            }
        }
        found
    }

    fn files(dir: &str, extension: &str) -> Vec<String> {
        fn walk(dir: &std::path::Path, extension: &str, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).expect("folder").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, extension, out);
                } else if path.extension().is_some_and(|e| e == extension) {
                    out.push(std::fs::read_to_string(&path).expect("source file"));
                }
            }
        }
        let mut out = Vec::new();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        walk(&root, extension, &mut out);
        out
    }

    #[test]
    fn every_text_in_the_rust_code_is_translated() {
        let mut keys: Vec<String> = files("src", "rs")
            .iter()
            .flat_map(|source| texts_after(source, "t!("))
            .collect();
        keys.sort();
        keys.dedup();
        // The scan works: the code has dozens of texts.
        assert!(keys.len() > 30, "only {} texts found", keys.len());
        for (lang, po) in [(Lang::It, IT_PO), (Lang::Es, ES_PO)] {
            let catalogue = parse_po(po);
            for key in &keys {
                let translation = catalogue
                    .get(key)
                    .unwrap_or_else(|| panic!("{}: no translation for {key:?}", lang.code()));
                assert_eq!(
                    placeholders(key),
                    placeholders(translation),
                    "{}: {key:?} and {translation:?} name different values",
                    lang.code()
                );
            }
            // No stale entries: a text removed from the code leaves the file.
            for key in catalogue.keys() {
                assert!(
                    keys.contains(key),
                    "{}: {key:?} is not in the code",
                    lang.code()
                );
            }
        }
    }

    #[test]
    fn every_text_in_the_windows_is_translated() {
        let keys: Vec<String> = files("ui", "slint")
            .iter()
            .flat_map(|source| texts_after(source, "@tr("))
            .collect();
        assert!(keys.len() > 40, "only {} texts found", keys.len());
        for code in ["it", "es"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("lang/{code}/LC_MESSAGES/knurl-probe-app.po"));
            let catalogue = parse_po(&std::fs::read_to_string(path).expect("catalogue"));
            for key in &keys {
                let translation = catalogue
                    .get(key)
                    .unwrap_or_else(|| panic!("{code}: no translation for {key:?}"));
                assert_eq!(
                    key.matches("{}").count(),
                    translation.matches("{}").count(),
                    "{code}: {key:?} and {translation:?} take a different number of values"
                );
            }
        }
    }

    #[test]
    fn the_same_word_is_translated_the_same_way_in_both_catalogues() {
        // A text that appears in the windows and in Rust has one translation.
        for (code, po) in [("it", IT_PO), ("es", ES_PO)] {
            let rust = parse_po(po);
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("lang/{code}/LC_MESSAGES/knurl-probe-app.po"));
            let windows = parse_po(&std::fs::read_to_string(path).expect("catalogue"));
            let mut shared = 0;
            for (key, translation) in &rust {
                if let Some(other) = windows.get(key) {
                    shared += 1;
                    assert_eq!(translation, other, "{code}: {key:?}");
                }
            }
            let _ = shared;
        }
    }

    #[test]
    fn translations_are_used() {
        assert_eq!(
            tr_in(Lang::It, "New model: thank you"),
            "Modello nuovo: grazie"
        );
        assert_eq!(
            tr_in(Lang::Es, "New model: thank you"),
            "Modelo nuevo: gracias"
        );
        assert_eq!(
            fill(
                tr_in(Lang::It, "Knurl already supports your {model}"),
                &[("model", &"WH-1000XM3")]
            ),
            "Knurl supporta già le tue WH-1000XM3"
        );
    }

    #[test]
    fn placeholders_are_listed() {
        assert_eq!(placeholders("{b} and {a}"), vec!["a", "b"]);
        assert!(placeholders("none").is_empty());
    }
}
