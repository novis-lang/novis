//! Nothing reads the system temp directory. Every temporary folder Novis makes lives under
//! `capability::temp_root`, which is `[io] temp_root` or the data folder's `tmp/`, and a test that
//! needs a directory asks `nvs_repo::scratch` for one under `target/`. This reads every Rust source
//! under `crates/` as text and fails on a call to `std::env::temp_dir()` anywhere, in test code and
//! product code alike.

use std::collections::BTreeMap;
use std::path::Path;

/// The tests whose subject is the system temp directory itself, and so keep calling it: the file,
/// how many calls it makes, and why.
const SUBJECT: &[(&str, usize, &str)] = &[];

#[test]
fn nothing_reads_the_system_temp_dir() {
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
        let code = code_only(&std::fs::read_to_string(file).unwrap());
        let at = calls(&code);
        if !at.is_empty() {
            found.insert(rel, at.len());
        }
    }
    let mut allowed = BTreeMap::new();
    for (file, calls, _why) in SUBJECT {
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
            (has != listed).then(|| format!("  {file}: {has} call(s), {listed} listed"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "code calls `std::env::temp_dir()`:\n{}\n\
         Product code makes its directories under `capability::temp_root`, through \
         `capability::private_dir` where there is no context. A test writes into \
         `nvs_repo::scratch(\"<name>\")`: a directory under `target/` that is deleted when its guard \
         drops. A test whose subject is the system temp directory itself goes in `SUBJECT` in {}, \
         with the reason. A file that makes fewer calls than `SUBJECT` lists has its entry lowered \
         or removed.",
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

/// Where `code` calls `env::temp_dir()`, as byte offsets.
fn calls(code: &str) -> Vec<usize> {
    code.match_indices("env::temp_dir()")
        .map(|(at, _)| at)
        .collect()
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
fn a_call_is_counted_and_a_comment_or_string_is_not() {
    let text = "pub fn temp_root() -> PathBuf {\n    std::env::temp_dir().join(\"novis\")\n}\n\
                // env::temp_dir() in a comment\n\
                fn product() { let s = \"env::temp_dir()\"; std::env::temp_dir(); }\n\
                #[cfg(test)]\nmod tests {\n    fn t() { env::temp_dir(); }\n}\n";
    assert_eq!(calls(&code_only(text)).len(), 3);
}
