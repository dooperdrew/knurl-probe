//! The design reference (design/README.md) says the UI uses only its tokens:
//! colours live in ui/theme.slint, nowhere else. This test fails when a
//! .slint file other than theme.slint writes a colour literal.

use std::path::Path;

/// Every `#rgb`, `#rrggbb` or `#rrggbbaa` literal in `text`, with its line.
fn colour_literals(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (n, line) in text.lines().enumerate() {
        // Comments may name a colour; only code counts.
        let code = line.split("//").next().unwrap_or("");
        let bytes = code.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'#' {
                let hex = bytes[i + 1..]
                    .iter()
                    .take_while(|b| b.is_ascii_hexdigit())
                    .count();
                if matches!(hex, 3 | 6 | 8) {
                    found.push((n + 1, code[i..i + 1 + hex].to_string()));
                }
                i += 1 + hex;
            } else {
                i += 1;
            }
        }
    }
    found
}

#[test]
fn colours_come_only_from_the_theme() {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(&ui).expect("ui folder") {
        let path = entry.expect("ui entry").path();
        let is_slint = path.extension().is_some_and(|e| e == "slint");
        if !is_slint || path.file_name().is_some_and(|n| n == "theme.slint") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable .slint file");
        for (line, lit) in colour_literals(&text) {
            offenders.push(format!("{}:{line}: {lit}", path.display()));
        }
    }
    assert!(
        offenders.is_empty(),
        "colour literals outside ui/theme.slint (use a Theme token):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_scanner_finds_literals_and_skips_comments() {
    let text = "a: #0B7A70;\n// #FFFFFF in a comment\nb: #fff; c: #12345; d: #11223344;";
    let found: Vec<String> = colour_literals(text).into_iter().map(|(_, s)| s).collect();
    assert_eq!(found, vec!["#0B7A70", "#fff", "#11223344"]);
}

/// `"name": "#rrggbb"` in one theme block of design/tokens.json.
fn token(block: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\":");
    let at = block.find(&key)? + key.len();
    let rest = block[at..].trim_start().strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

#[test]
fn the_theme_matches_the_design_tokens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(root.join("../../design/tokens.json")).expect("tokens.json");
    let light_at = json.find("\"light\"").expect("light block");
    let dark_at = json.find("\"dark\"").expect("dark block");
    let (light, dark) = (&json[light_at..dark_at], &json[dark_at..]);
    let dark = &dark[..dark.find('}').expect("end of dark block")];
    let theme = std::fs::read_to_string(root.join("ui/theme.slint")).expect("theme.slint");
    let mut checked = 0;
    for line in theme.lines() {
        let Some(rest) = line.trim().strip_prefix("out property <color> ") else {
            continue;
        };
        let (name, value) = rest.split_once(':').expect("name: value");
        let value = value.trim().trim_end_matches(';');
        let (d, l) = value
            .strip_prefix("dark ? ")
            .and_then(|v| v.split_once(" : "))
            .unwrap_or_else(|| panic!("{name}: expected `dark ? #dark : #light`"));
        assert_eq!(Some(l.to_string()), token(light, name), "light {name}");
        assert_eq!(Some(d.to_string()), token(dark, name), "dark {name}");
        checked += 1;
    }
    assert!(
        checked >= 20,
        "only {checked} colour tokens found in theme.slint"
    );
}

/// `name: dark ? #DARK : #LIGHT;` lines of ui/theme.slint, as
/// (name, light, dark), upper-case hex.
fn theme_colours(text: &str) -> Vec<(String, String, String)> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("out property <color> ")?;
            let (name, value) = rest.split_once(':')?;
            let (_, choices) = value.split_once('?')?;
            let (dark, light) = choices.split_once(':')?;
            let clean = |s: &str| s.trim().trim_end_matches(';').trim().to_uppercase();
            Some((name.trim().to_string(), clean(light), clean(dark)))
        })
        .collect()
}

/// The `"name": "#RRGGBB"` pairs of one theme block of design/tokens.json.
fn token_colours(json: &str, theme: &str) -> Vec<(String, String)> {
    let start = json
        .find(&format!("\"{theme}\": {{"))
        .expect("theme block in tokens.json");
    let block = &json[start..];
    let block = &block[..block.find('}').expect("end of theme block")];
    block
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (name, value) = line.trim().trim_end_matches(',').split_once(':')?;
            Some((
                name.trim().trim_matches('"').to_string(),
                value.trim().trim_matches('"').to_uppercase(),
            ))
        })
        .collect()
}

#[test]
fn the_theme_has_exactly_the_design_tokens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let theme = std::fs::read_to_string(root.join("ui/theme.slint")).expect("theme.slint");
    let tokens =
        std::fs::read_to_string(root.join("../../design/tokens.json")).expect("tokens.json");
    let in_theme = theme_colours(&theme);
    let light = token_colours(&tokens, "light");
    let dark = token_colours(&tokens, "dark");
    assert_eq!(
        in_theme.len(),
        light.len(),
        "every light token is in the theme"
    );
    assert_eq!(
        in_theme.len(),
        dark.len(),
        "every dark token is in the theme"
    );
    for (name, l, d) in &in_theme {
        let want_l = light.iter().find(|(n, _)| n == name).map(|(_, v)| v);
        let want_d = dark.iter().find(|(n, _)| n == name).map(|(_, v)| v);
        assert_eq!(want_l, Some(l), "{name} (light)");
        assert_eq!(want_d, Some(d), "{name} (dark)");
    }
}
