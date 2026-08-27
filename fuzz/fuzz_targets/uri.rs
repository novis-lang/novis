//! Fuzz target: `Core\Uri::parse` must not panic on any text, and whatever it
//! accepts must round-trip — `parse` → `toString` → `parse` answers a URI that
//! `compareTo` says is the same one.
//!
//! The property is the interesting half. `parse` reports every component
//! verbatim while `compareTo` normalizes both sides per RFC 3986 § 6.2.2, so a
//! round trip that lost or moved a component would show up as a non-zero
//! answer here even though the text still parses — which is exactly the class
//! of bug re-parsing alone cannot see (`nvs_stdlib::uri`'s `unmoved`).
//!
//! Run with `cargo +nightly fuzz run uri` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_runtime::{Ctx, NvsStr, OutputSink, Value, call};
use nvs_stdlib::uri::{nvs_core_uri_compare_to, nvs_core_uri_parse, nvs_core_uri_to_string};

/// `Core\Uri::parse($text)`, or `None` where the grammar refused it — which is
/// most inputs, and not a failure.
fn parse(ctx: &mut Ctx, text: &str) -> Option<Value> {
    let argument = Value::str(NvsStr::new(text.as_bytes()));
    let answer = call(nvs_core_uri_parse, ctx, &[argument]).ok();
    unsafe { argument.release() };
    answer
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut ctx = Ctx::new(OutputSink::Sink);
    let Some(once) = parse(&mut ctx, text) else {
        return;
    };

    let rendered = call(nvs_core_uri_to_string, &mut ctx, &[once])
        .expect("`toString` reads a slot and never throws");
    let rendered_text = String::from_utf8(
        rendered
            .as_str_bytes()
            .expect("`toString` answers a `string`")
            .to_vec(),
    )
    .expect("ADR 0009 guarantees a `string` is UTF-8");
    assert_eq!(rendered_text, text, "`toString` answers the text parsed");

    let twice = parse(&mut ctx, &rendered_text).expect("text that parsed once parses again");
    let ordering = call(nvs_core_uri_compare_to, &mut ctx, &[once, twice])
        .expect("comparing two `Uri`s never throws")
        .as_int()
        .expect("`compareTo` answers an `int`");
    assert_eq!(ordering, 0, "a URI is equivalent to its own re-parsed text");

    unsafe {
        rendered.release();
        twice.release();
        once.release();
    }
});
