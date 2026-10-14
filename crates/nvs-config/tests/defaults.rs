//! The shipped `default.toml` against the built-in defaults table, so the two cannot drift apart.
//!
//! A line marked `# default` shows the value an unset key has, so its key has a fixed default in
//! [`nvs_config::defaults::DEFAULTS`] and the line shows exactly what that row renders. A line
//! marked `# default: <text>` or `# example` says no single value is the default, so its key has no
//! fixed default there. And every row of the table is a key the file shows.

use std::collections::BTreeSet;

use nvs_config::defaults::{self, At, DEFAULTS, Value};

/// One setting line of the shipped file.
struct Setting {
    /// The dotted key, under the nearest header above it, live or commented out.
    key: String,
    /// The value written beside the key, as the file spells it.
    value: String,
    /// The note after the value, without `; restart required`.
    note: String,
    /// The line number, from one.
    line: usize,
}

/// Every setting line of the shipped file, in order.
fn settings() -> Vec<Setting> {
    let mut block = String::new();
    let mut out = Vec::new();
    for (index, raw) in nvs_config::default_file().lines().enumerate() {
        let line = raw.trim();
        let setting = line.strip_prefix('#').unwrap_or(line);
        if let Some(header) = setting.strip_prefix('[') {
            block = header
                .trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or_default()
                .to_string();
            continue;
        }
        let Some((name, rest)) = setting.split_once('=') else {
            continue;
        };
        let name = name.trim_end();
        let bare = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_';
        if name.is_empty() || !name.bytes().all(bare) {
            continue;
        }
        let (value, note) = split(rest);
        out.push(Setting {
            key: if block.is_empty() {
                name.to_string()
            } else {
                format!("{block}.{name}")
            },
            value,
            note: note
                .trim_end_matches("; restart required")
                .trim()
                .to_string(),
            line: index + 1,
        });
    }
    out
}

/// The value and the note of what follows a setting's `=`: the note starts at the first `#` that
/// is not inside a string.
fn split(rest: &str) -> (String, String) {
    let mut quote = None;
    for (at, c) in rest.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' => {
                return (
                    rest[..at].trim().to_string(),
                    rest[at + 1..].trim().to_string(),
                );
            }
            None => {}
        }
    }
    (rest.trim().to_string(), String::new())
}

/// The key of the row a setting line is the default of: an `[[app]]` block's keys are the global
/// block's own keys, folded over them, and a `[[schedule]]` entry's `limits` are its ceilings, the
/// keys of `[limits.hard]`.
fn row_key(key: &str) -> String {
    match key.strip_prefix("schedule.limits.") {
        Some(leaf) => format!("limits.hard.{leaf}"),
        None => key.strip_prefix("app.").unwrap_or(key).to_string(),
    }
}

/// A fixed value as the shipped file writes it, quotes included, or `None` for a computed one.
fn fixed(value: &Value) -> Option<String> {
    let table = toml::Table::new();
    let at = At {
        table: &table,
        serving: true,
        mode: "production",
        started: "production",
    };
    match value {
        Value::List(items) => Some(defaults::render_list(items)),
        Value::Text(_) | Value::Time(_) | Value::Size(_) | Value::FileMode(_) => {
            defaults::render(value, "", &at).map(|text| format!("\"{text}\""))
        }
        Value::Count(_) | Value::Ratio(_) | Value::Flag(_) => defaults::render(value, "", &at),
        _ => None,
    }
}

/// Every `# default` line shows the value its row renders, every other line's key has no fixed
/// default, and every row is a key the file shows.
#[test]
fn every_default_line_shows_the_value_the_table_ships() {
    let mut problems = Vec::new();
    let mut reached = BTreeSet::new();
    for setting in settings() {
        let row = defaults::row(&row_key(&setting.key));
        if let Some(row) = row {
            reached.insert(row.key);
        }
        let shipped = row.and_then(|row| fixed(&row.value));
        let at = format!("default.toml:{} `{}`", setting.line, setting.key);
        match (setting.note.as_str(), shipped) {
            ("default", Some(shipped)) if shipped == setting.value => {}
            ("default", Some(shipped)) => problems.push(format!(
                "{at} shows {}, and the table ships {shipped}",
                setting.value
            )),
            ("default", None) => problems.push(format!(
                "{at} is marked `# default`, and the table has no fixed default for it"
            )),
            (_, Some(shipped)) => problems.push(format!(
                "{at} is marked `# {}`, and the table ships {shipped}: show it and mark it `# default`",
                setting.note
            )),
            (_, None) => {}
        }
    }
    for row in DEFAULTS {
        if !reached.contains(row.key) {
            problems.push(format!(
                "the table has `{}`, and default.toml shows no such key",
                row.key
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// A size, a duration and a mode render the way the shipped file writes them, so `get` and the
/// file agree on the text as well as the number.
#[test]
fn a_default_renders_the_way_the_file_writes_it() {
    let table = toml::Table::new();
    let at = At {
        table: &table,
        serving: true,
        mode: "production",
        started: "production",
    };
    let render = |value| defaults::render(&value, "", &at);
    assert_eq!(
        render(Value::Size(defaults::LIMITS_MEMORY)).as_deref(),
        Some("128M")
    );
    assert_eq!(
        render(Value::Time(defaults::LIMITS_CPU_TIME)).as_deref(),
        Some("30s")
    );
    assert_eq!(
        render(Value::Time(defaults::LIMITS_FATAL_RESERVE_TIME)).as_deref(),
        Some("50ms")
    );
    assert_eq!(
        render(Value::Time(defaults::SERVER_CONNECTION_MAX_LIFETIME)).as_deref(),
        Some("24h")
    );
    assert_eq!(
        render(Value::Time(defaults::HTTP_HEADERS_HSTS)).as_deref(),
        Some("365d")
    );
    assert_eq!(
        render(Value::FileMode(defaults::SERVER_SOCKET_MODE)).as_deref(),
        Some("0660")
    );
    assert_eq!(
        render(Value::Ratio(defaults::TRACE_SAMPLE)).as_deref(),
        Some("0.0")
    );
}
