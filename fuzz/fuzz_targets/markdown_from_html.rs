//! Fuzz target: `Core\Markdown::fromHtml`.
//!
//! The check is `nvs_fuzz_checks::markdown_from_html`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz markdown_from_html --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::markdown_from_html(data));
