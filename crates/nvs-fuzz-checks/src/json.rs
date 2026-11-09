use nvs_runtime::{Ctx, OutputSink, Value};
use nvs_stdlib::fuzz::{Owned, call};
use nvs_stdlib::json::{nvs_core_json_decode, nvs_core_json_encode};

/// The deepest document `decode` accepts, so the fuzzer reaches the ceiling.
const MAX_DEPTH: u64 = 1024;

/// `Core\Json::decode($text, {maxDepth: MAX_DEPTH})`, or `None` where it threw.
fn decode(ctx: &mut Ctx, text: &str) -> Option<Owned> {
    let argument = Owned::text(text);
    call(
        nvs_core_json_decode,
        ctx,
        &[argument.get(), Value::uint(MAX_DEPTH)],
    )
}

/// `Core\Json::encode($value)`'s text, or `None` where it threw.
fn encode(ctx: &mut Ctx, value: &Owned) -> Option<String> {
    let written = call(
        nvs_core_json_encode,
        ctx,
        &[value.get(), Value::bool(false), Value::bool(false)],
    )?;
    let bytes = written
        .get()
        .as_str_bytes()
        .expect("`encode` answers a `string`")
        .to_vec();
    Some(
        String::from_utf8(bytes)
            .expect("`rule:types/string-is-utf8` guarantees a `string` is UTF-8"),
    )
}

/// `Core\Json::decode` must not panic on any text, and whatever it accepts must
/// settle after one round trip: `encode` of the decoded value, decoded and
/// encoded again, gives the same text.
///
/// One round is allowed to change the text, because `encode` writes a `float`
/// in its own shortest form and drops the whitespace. A second round that
/// changed it again would mean a value `decode` built is not the value it read.
/// `decode` is also where a repeated key is refused
/// (`rule:errors/ambiguous-input-refused`), so that refusal is fuzzed here too.
pub fn json(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut ctx = Ctx::new(OutputSink::Sink);
    let Some(once) = decode(&mut ctx, text) else {
        return;
    };
    // A decoded value `encode` cannot write is not this check's question:
    // `decode` builds no `float` that is not finite, and that is all it refuses.
    let Some(first) = encode(&mut ctx, &once) else {
        return;
    };
    let twice = decode(&mut ctx, &first).expect("text `encode` wrote decodes again");
    let second = encode(&mut ctx, &twice).expect("a value that encoded once encodes again");
    assert_eq!(first, second, "one round trip settles the text");
}
