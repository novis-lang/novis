//! Fuzz target: `Core\Mail\Message::parse`, the reader of a received message.
//!
//! The check is `nvs_fuzz_checks::mail_parse`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz mail_parse --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::mail_parse(data));
