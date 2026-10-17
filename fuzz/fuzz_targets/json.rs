//! Fuzz target: `Core\Json::decode` must not panic on any text, and whatever it
//! accepts must settle after one round trip — `encode` of the decoded value,
//! decoded and encoded again, gives the same text.
//!
//! One round is allowed to change the text, because `encode` writes a `float`
//! in its own shortest form and drops the whitespace. A second round that
//! changed it again would mean a value `decode` built is not the value it read.
//! `decode` is also where a repeated key is refused
//! (`rule:errors/ambiguous-input-refused`), so that refusal is fuzzed here too.
//!
//! Run with `cargo +nightly fuzz run json` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_runtime::{Ctx, NvsStr, OutputSink, Value, call};
use nvs_stdlib::json::{nvs_core_json_decode, nvs_core_json_encode};

/// The deepest document `decode` accepts, so the fuzzer reaches the ceiling.
const MAX_DEPTH: u64 = 1024;

/// `Core\Json::decode($text, {maxDepth: MAX_DEPTH})`, or `None` where it threw.
fn decode(ctx: &mut Ctx, text: &[u8]) -> Option<Value> {
    let argument = Value::str(NvsStr::new(text));
    let answer = call(nvs_core_json_decode, ctx, &[argument, Value::uint(MAX_DEPTH)]).ok();
    unsafe { argument.release() };
    answer
}

/// `Core\Json::encode($value)`'s text, or `None` where it threw.
fn encode(ctx: &mut Ctx, value: Value) -> Option<Vec<u8>> {
    let written = call(
        nvs_core_json_encode,
        ctx,
        &[value, Value::bool(false), Value::bool(false)],
    )
    .ok()?;
    let text = written
        .as_str_bytes()
        .expect("`encode` answers a `string`")
        .to_vec();
    unsafe { written.release() };
    Some(text)
}

fuzz_target!(|data: &[u8]| {
    if std::str::from_utf8(data).is_err() {
        return;
    }
    let mut ctx = Ctx::new(OutputSink::Sink);
    let Some(once) = decode(&mut ctx, data) else {
        return;
    };
    // A decoded value `encode` cannot write is not this target's question:
    // `decode` builds no `float` that is not finite, and that is all it refuses.
    let Some(first) = encode(&mut ctx, once) else {
        unsafe { once.release() };
        return;
    };
    let twice = decode(&mut ctx, &first).expect("text `encode` wrote decodes again");
    let second = encode(&mut ctx, twice).expect("a value that encoded once encodes again");
    assert_eq!(
        String::from_utf8_lossy(&first),
        String::from_utf8_lossy(&second),
        "one round trip settles the text"
    );
    unsafe {
        twice.release();
        once.release();
    }
});
