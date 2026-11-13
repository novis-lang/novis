use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

/// A target's name, the directory its seeds are in, and its check.
type Target = (&'static str, &'static str, fn(&[u8]));

/// Each target with a check, the directory under `fuzz/seeds/` its inputs are
/// in, and the check. `ast` reads `parse`'s seeds, since both take one source
/// text, and `json_reader` reads `json`'s, since both take one JSON text.
const TARGETS: [Target; 18] = [
    ("ast", "parse", crate::ast),
    ("cookie", "cookie", crate::cookie),
    ("decompress", "decompress", crate::decompress),
    ("image_path", "image_path", crate::image_path),
    ("json", "json", crate::json),
    ("json_reader", "json", crate::json_reader),
    ("lex", "lex", crate::lex),
    ("markdown", "markdown", crate::markdown),
    (
        "markdown_from_html",
        "markdown_from_html",
        crate::markdown_from_html,
    ),
    ("multipart", "multipart", crate::multipart),
    ("parse", "parse", crate::parse),
    ("prefix", "prefix", crate::prefix),
    ("regex", "regex", crate::regex),
    ("reply_head", "reply_head", crate::reply_head),
    ("request_head", "request_head", crate::request_head),
    ("serialize", "serialize", crate::serialize),
    ("uri", "uri", crate::uri),
    ("xml", "xml", crate::xml),
];

/// Every file under `fuzz/seeds/<dir>/`, sorted. A directory that does not
/// exist holds no inputs.
fn seeds(dir: &str) -> Vec<PathBuf> {
    let dir = nvs_repo::path(&format!("fuzz/seeds/{dir}"));
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut seeds: Vec<PathBuf> = entries
        .map(|entry| entry.expect("a readable seed").path())
        .filter(|path| path.is_file())
        .collect();
    seeds.sort();
    seeds
}

/// The text a caught panic carries.
fn message(cause: &(dyn std::any::Any + Send)) -> &str {
    cause
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| cause.downcast_ref::<&str>().copied())
        .unwrap_or("a panic with no message")
}

#[test]
#[cfg_attr(
    miri,
    ignore = "parses, renders and decodes every saved input, which Miri takes minutes for"
)]
fn every_saved_fuzz_input_passes_its_targets_check() {
    let mut checked = 0usize;
    let mut failed = Vec::new();
    for (target, dir, check) in TARGETS {
        for seed in seeds(dir) {
            let data = std::fs::read(&seed)
                .unwrap_or_else(|why| panic!("{} is readable: {why}", seed.display()));
            checked += 1;
            if let Err(cause) = catch_unwind(AssertUnwindSafe(|| check(&data))) {
                failed.push(format!(
                    "{} fails the `{target}` check: {}",
                    seed.display(),
                    message(&*cause)
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    // `fuzz/seeds/` is committed, so finding none means the path is wrong.
    assert!(checked > 0, "no seed was found under fuzz/seeds/");
}
