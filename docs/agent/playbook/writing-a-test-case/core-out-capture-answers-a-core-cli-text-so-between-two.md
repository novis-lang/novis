- **`Core\Out::capture` and a script result's `output` answer a carrier object, so `==` between two
  captures is identity and a case comparing them counts zero agreements while printing the right
  bytes.**
  `rule:expressions/object-identity-equality` makes `==` on two objects identity, and the
  neighbouring `Core\Response` cases hide it by only ever printing a capture. Interpolate each into
  a `string` first — `string $s = "{$captured}";` renders the carrier through
  `rule:security/capture-answers-the-carrier` — and `==` then compares content.
  [until: gone crates/nvs-stdlib/src/out.rs:Core\Out::capture]
