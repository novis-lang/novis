- **`mixed as string` does not reach a `bytes`; the failure is a runtime throw inside the walk, not
  a diagnostic at the line.** `value_to_string` refuses `Tag::Bytes` as `rule:types/conversion`
  requires and `mixed` gives it no second chance, so `$value as string` compiles and throws `cannot
  convert a `bytes` value to `string`` on the first leaf. Write `$value as bytes as string` — narrow
  the `mixed` to the type it holds, then take the checked row; and `Core\Json::encode` refuses an
  array holding a `bytes`, so a member that starts answering octets invalidates every JSON-rendering
  fixture. [until: reviewed 2026-09-06]
