//! No test writes into the system temp directory: a test that needs a directory asks
//! `nvs_repo::scratch` for one under `target/`. This reads every test source under `crates/` as
//! text and fails on a call to `std::env::temp_dir()` that neither list below names.
//!
//! Test code is a file under a package's `tests/`, a file a `#[cfg(test)] mod name;` declares, a
//! file that opens with `#![cfg(test)]`, and the item that follows a `#[cfg(…)]` naming `test`.

use std::collections::BTreeMap;
use std::path::Path;

/// The tests whose subject is the system temp directory itself, and so keep calling it: the file,
/// how many calls it makes in test code, and why.
const SUBJECT: &[(&str, usize, &str)] = &[];

/// The calls in test code that have not moved to `nvs_repo::scratch` yet, by file. The guard fails
/// when a file makes more calls than it is listed with, and also when it makes fewer, so the list
/// only shrinks.
const NOT_YET_MOVED: &[(&str, usize)] = &[
    ("crates/nvs-config/tests/control.rs", 1),
    ("crates/nvs-db/src/mysql.rs", 2),
    ("crates/nvs-db/src/pg.rs", 1),
    ("crates/nvs-footprint/src/lib.rs", 1),
    ("crates/nvs-hir/src/requires.rs", 1),
    ("crates/nvs-host/src/net.rs", 1),
    ("crates/nvs-host/src/tls.rs", 2),
    ("crates/nvs-runtime/src/capability.rs", 2),
    ("crates/nvs-runtime/src/ctx/hooks.rs", 1),
    ("crates/nvs-runtime/src/deferred.rs", 1),
    ("crates/nvs-runtime/src/sweep.rs", 3),
    ("crates/nvs-server/src/serve.rs", 1),
    ("crates/nvs-stdlib/src/cache.rs", 2),
    ("crates/nvs-stdlib/src/cache/redis.rs", 1),
    ("crates/nvs-stdlib/src/csv.rs", 1),
    ("crates/nvs-stdlib/src/http.rs", 2),
    ("crates/nvs-stdlib/src/http/stream.rs", 1),
    ("crates/nvs-stdlib/src/http/transport.rs", 1),
    ("crates/nvs-stdlib/src/io.rs", 1),
    ("crates/nvs-stdlib/src/lib.rs", 1),
    ("crates/nvs-stdlib/src/log.rs", 1),
    ("crates/nvs-stdlib/src/net.rs", 1),
    ("crates/nvs-stdlib/src/queue.rs", 1),
    ("crates/nvs-stdlib/src/request.rs", 1),
    ("crates/nvs-stdlib/src/response.rs", 1),
    ("crates/nvs-stdlib/src/storage.rs", 1),
    ("crates/nvs-stdlib/src/zip.rs", 1),
    ("crates/nvs-stdlib/tests/capability.rs", 1),
    ("crates/nvs-test/src/run.rs", 3),
    ("crates/nvs-types/tests/common/mod.rs", 1),
];

#[test]
fn no_test_code_writes_into_the_system_temp_dir() {
    let root = nvs_repo::path("crates");
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut found = BTreeMap::new();
    for file in &files {
        let rel = format!(
            "crates/{}",
            file.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        );
        let text = std::fs::read_to_string(file).unwrap();
        let whole = rel.split('/').nth(2) == Some("tests") || test_only(&files, file);
        let calls = calls_in_test_code(&text, whole);
        if calls > 0 {
            found.insert(rel, calls);
        }
    }
    let mut allowed = BTreeMap::new();
    for (file, calls, _why) in SUBJECT {
        *allowed.entry((*file).to_string()).or_insert(0) += calls;
    }
    for (file, calls) in NOT_YET_MOVED {
        *allowed.entry((*file).to_string()).or_insert(0) += calls;
    }
    let wrong: Vec<String> = found
        .keys()
        .chain(allowed.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter_map(|file| {
            let has = found.get(file).copied().unwrap_or(0);
            let listed = allowed.get(file).copied().unwrap_or(0);
            (has != listed)
                .then(|| format!("  {file}: {has} call(s) in test code, {listed} listed"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "test code calls `std::env::temp_dir()` where nothing lists it:\n{}\n\
         Write into `nvs_repo::scratch(\"<name>\")` instead: it is a directory under `target/` that is \
         deleted when its guard drops. A test whose subject is the system temp directory itself goes \
         in `SUBJECT` in {}, with the reason. A file that makes fewer calls than it is listed with \
         has its entry lowered or removed.",
        wrong.join("\n"),
        file!()
    );
}

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if entry.file_name() != "target" {
                walk(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Whether `file` is declared by a `#[cfg(test)] mod name;` in the file that owns its directory.
fn test_only(files: &[std::path::PathBuf], file: &Path) -> bool {
    let stem = file.file_stem().unwrap().to_string_lossy();
    let (name, dir) = if stem == "mod" {
        let dir = file.parent().unwrap();
        (
            dir.file_name().unwrap().to_string_lossy(),
            dir.parent().unwrap(),
        )
    } else {
        (stem, file.parent().unwrap())
    };
    let parents = [
        dir.join("lib.rs"),
        dir.join("main.rs"),
        dir.join("mod.rs"),
        dir.with_extension("rs"),
    ];
    parents.iter().filter(|p| files.contains(p)).any(|parent| {
        let text = std::fs::read_to_string(parent).unwrap();
        let code = code_only(&text);
        test_items(&code).iter().any(|item| {
            let decl: String = code[item.0..item.1].split_whitespace().collect();
            decl == format!("mod{name};") || decl == format!("pubmod{name};")
        })
    })
}

/// How many calls to `env::temp_dir()` the text makes in test code. `whole` says the whole file is.
fn calls_in_test_code(text: &str, whole: bool) -> usize {
    let code = code_only(text);
    let whole = whole || code.trim_start().starts_with("#![cfg(test)]");
    let items = test_items(&code);
    code.match_indices("env::temp_dir()")
        .filter(|(at, _)| whole || items.iter().any(|item| (item.0..item.1).contains(at)))
        .count()
}

/// The byte ranges of the items that follow a `#[cfg(…)]` naming `test`, each up to its closing
/// brace or its `;`.
fn test_items(code: &str) -> Vec<(usize, usize)> {
    let bytes = code.as_bytes();
    let mut items = Vec::new();
    let mut from = 0;
    while let Some(found) = code[from..].find("#[cfg(") {
        let start = from + found;
        let close = code[start..].find(']').map_or(code.len(), |i| start + i);
        from = close;
        let mut words = code[start..close].split(|c: char| !c.is_alphanumeric() && c != '_');
        let names_test = words.clone().any(|word| word == "test") && !words.any(|w| w == "not");
        if !names_test {
            continue;
        }
        let Some(open) = code[close..].find(['{', ';']).map(|i| close + i) else {
            break;
        };
        let item_start = close + 1;
        if bytes[open] == b';' {
            items.push((item_start, open + 1));
            continue;
        }
        let mut depth = 0usize;
        let mut end = code.len();
        for (i, &b) in bytes.iter().enumerate().skip(open) {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        items.push((item_start, end));
        from = end;
    }
    items
}

/// The text with every comment, string and character literal replaced by spaces of the same
/// length, so a brace or a call inside one is never counted.
fn code_only(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let blank = |out: &mut String, from: &[char]| {
        for c in from {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        }
    };
    let mut i = 0;
    while i < chars.len() {
        let rest = &chars[i..];
        let len = if rest.starts_with(&['/', '/']) {
            rest.iter().position(|&c| c == '\n').unwrap_or(rest.len())
        } else if rest.starts_with(&['/', '*']) {
            let mut depth = 0;
            let mut j = 0;
            while j < rest.len() {
                if rest[j..].starts_with(&['/', '*']) {
                    depth += 1;
                    j += 2;
                } else if rest[j..].starts_with(&['*', '/']) {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            j
        } else if rest[0] == 'r'
            && (i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'))
            && rest.get(1).is_some_and(|&c| c == '"' || c == '#')
        {
            let hashes = rest[1..].iter().take_while(|&&c| c == '#').count();
            if rest.get(1 + hashes) == Some(&'"') {
                let body = 2 + hashes;
                let close: Vec<char> = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                (body..rest.len())
                    .find(|&j| rest[j..].starts_with(&close))
                    .map_or(rest.len(), |j| j + close.len())
            } else {
                0
            }
        } else if rest[0] == '"' {
            let mut j = 1;
            while j < rest.len() && rest[j] != '"' {
                j += if rest[j] == '\\' { 2 } else { 1 };
            }
            (j + 1).min(rest.len())
        } else if rest[0] == '\'' {
            if rest.get(1) == Some(&'\\') {
                rest[2..]
                    .iter()
                    .position(|&c| c == '\'')
                    .map_or(1, |j| j + 3)
            } else if rest.get(2) == Some(&'\'') {
                3
            } else {
                0
            }
        } else {
            0
        };
        if len == 0 {
            out.push(chars[i]);
            i += 1;
        } else {
            blank(&mut out, &rest[..len]);
            i += len;
        }
    }
    out
}

#[test]
fn a_call_in_a_test_module_is_counted_and_one_in_product_code_is_not() {
    let text = "fn product() { std::env::temp_dir(); }\n\
                // env::temp_dir() in a comment\n\
                #[cfg(test)]\nmod tests {\n    fn t() { let s = \"}\"; std::env::temp_dir(); }\n}\n\
                fn after() { env::temp_dir(); }\n\
                #[cfg(not(test))]\nfn product_only() { env::temp_dir(); }\n";
    assert_eq!(calls_in_test_code(text, false), 1);
    assert_eq!(calls_in_test_code(text, true), 4);
}
