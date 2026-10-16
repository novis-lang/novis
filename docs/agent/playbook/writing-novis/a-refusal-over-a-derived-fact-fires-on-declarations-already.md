- **A refusal over a derived fact fires on declarations already refused for something else, and the
  existing `--EXPECTF-ERROR--` case is what catches it.**
  Refusing a `#[Json\Derive]` class whose field list came out empty also hit
  `reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst`, whose properties are both refused. Before adding a diagnostic whose condition
  is the absence of a result, grep the reject tree for a case that already makes that result absent,
  and model the outcome per item (kept / skipped / refused) rather than as an `Option`.
  `until: gone tests/conformance/reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst:--EXPECTF-ERROR--`
