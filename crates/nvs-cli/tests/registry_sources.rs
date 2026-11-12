//! Checks over the source of the registry application under `apps/registry/`.

use std::fs;
use std::path::Path;

/// The folders whose `.nvs` files hold the registry's transactions.
const FOLDERS: [&str; 2] = ["apps/registry/src", "apps/registry/jobs"];

/// The statements that only read the database.
const READS: [&str; 4] = ["->query(", "->queryAs(", "->stream(", "->streamAs("];

/// The statements that write to the database.
const WRITES: [&str; 2] = ["->execute(", "->executeMany("];

/// Every transaction of the registry that writes starts with a statement that writes. SQLite
/// gives a transaction that first reads a shared lock, and the first write then fails at once
/// with `SQLITE_BUSY` while another request writes, with no wait. A transaction that only reads,
/// such as the backup's snapshot, is not named.
#[test]
fn every_registry_transaction_starts_with_a_write() {
    let mut found = Vec::new();
    let mut seen = 0;
    for folder in FOLDERS {
        let mut files: Vec<_> = fs::read_dir(nvs_repo::path(folder))
            .unwrap_or_else(|error| panic!("{folder} is read: {error}"))
            .map(|entry| entry.expect("a folder entry is read").path())
            .filter(|path| is_source(path))
            .collect();
        files.sort();
        for file in files {
            let text = fs::read_to_string(&file).expect("a source file is read");
            for (line, body) in transactions(&text) {
                seen += 1;
                let first_read = first_of(body, &READS);
                let first_write = first_of(body, &WRITES);
                if let (Some(read), Some(write)) = (first_read, first_write)
                    && read < write
                {
                    let name = file.file_name().expect("a file name").to_string_lossy();
                    found.push(format!("{folder}/{name}:{line}"));
                }
            }
        }
    }
    assert!(
        seen >= 10,
        "only {seen} transactions were found, so the scan no longer reads them"
    );
    assert!(
        found.is_empty(),
        "these transactions read before they write. Start each with a statement that writes, as \
         `Releases::rebind` does:\n{}",
        found.join("\n")
    );
}

/// A `.nvs` file that is not a test class.
fn is_source(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.ends_with(".nvs") && !name.ends_with("Test.nvs")
}

/// Each `transaction(` call in `text`, with its line and the body of the function it is given.
fn transactions(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    for (at, _) in text.match_indices("->transaction(") {
        let line = text[..at].matches('\n').count() + 1;
        let rest = &text[at..];
        let Some(open) = rest.find("=> {") else {
            panic!("the transaction at line {line} has no function body the scan can read");
        };
        let body = &rest[open + 3..];
        let mut depth = 0usize;
        let mut end = body.len();
        for (index, byte) in body.bytes().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = index;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push((line, &body[..end]));
    }
    out
}

/// The offset of the first of `needles` in `body`, if any is there.
fn first_of(body: &str, needles: &[&str]) -> Option<usize> {
    needles.iter().filter_map(|needle| body.find(needle)).min()
}
