/// The most output a decode may give, in octets.
const BYTES: u64 = 1 << 16;
/// The most output a decode may give per octet of input.
const RATIO: u64 = 64;

/// Every `Core\Compress` decoder must not panic on any frame, and never gives
/// more output than its bound allows (`rule:core-classes/decompression-bound`).
///
/// The first byte picks the codec and the rest is the frame. The bound is
/// small, so the fuzzer reaches the refusal of a frame that expands too far.
pub fn decompress(data: &[u8]) {
    let Some((&codec, frame)) = data.split_first() else {
        return;
    };
    if let Some(out) = nvs_stdlib::fuzz::decompress(codec, frame, BYTES, RATIO) {
        let ceiling = BYTES.min(RATIO * frame.len() as u64);
        assert!(out.len() as u64 <= ceiling, "the output passed its bound");
    }
}
