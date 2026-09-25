- **A class carrying `#[Json\Derive]` and declaring no field has an empty codec, so
  `Core\Json::encode`/`decodeAs<T>` refuse it with the sentence saying it does not carry the
  attribute at all.** The message is wrong about why, and whether a fieldless class should encode as
  `{}` instead is the open question behind it. Do not pin that sentence in a case over a fieldless
  class. [until: gone crates/nvs-stdlib/src/json.rs:does not carry]
