- **A conformance case can quote a known gap's own refusal message, so closing the gap fails a case
  whose name says nothing about it.** A gap's message is observable behaviour while the gap stands,
  so pinning it was right — `json-an-absent-optional-field-reads-apart-from-an-absent-required-one`
  carried "…is `nvs_stdlib::json`'s own known gap" in its `--EXPECT--`. Before verifying a closed
  gap, grep `tests/` for a phrase of the message you are deleting, and rewrite the case holding
  it to assert the new behaviour. [until: gone tests/conformance/core/json-an-absent-optional-field-reads-apart-from-an-absent-required-one.nvst:--EXPECT--]
