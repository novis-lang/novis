- **`Core\Request::route()` throws a `LogicError` where there is no request, and a `.nvst` case that
  catches `RuntimeError` around it does not catch.** The runner fixture `in-process-request.nvs`
  catches `RuntimeError` and is green only because `nvs test` runs the entry with a request in
  front; the message "there is no request here" identifies it, the trace frame does not. Catch
  `LogicError`; `rule:security/request-state-throws-in-an-isolate` says why.
  [until: reviewed 2026-09-06]
