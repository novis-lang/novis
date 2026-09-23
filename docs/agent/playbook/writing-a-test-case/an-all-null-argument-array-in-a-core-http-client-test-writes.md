- **An all-`null` argument array in a `Core\Http\Client` test writes a body.** `[Value::null();
  REQUEST_ARITY]` leaves the `json` slot holding `Tag::Null`, and that key omits as
  `Const::NeverWritten`, so `judge_verb` refuses the call as a `GET` carrying a body before the case
  reaches the thing it is about. Write `args[JSON] = Value::unset()` in any request test that is not
  about the body. [until: gone crates/nvs-stdlib/src/http.rs:Const::NeverWritten]
