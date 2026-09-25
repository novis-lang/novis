- **A `bun nv gaps --errors` row marked `thrown_as` carries a *class*, and asserting which one is the
  row's point.** `Fault::thrown_as(ThrownClass::Logic, …)` and `…::Parse` are ordinary `Throwable`s,
  so `catch (Throwable $e)` reaches both and says nothing; the discriminating clause is `catch
  (LogicError $e)` or `catch (ParseError $e)`, and two on one `try` compile at file scope with
  separate binding names. The roster is `nvs_runtime::throwable::ThrownClass`'s doc comment.
  [until: gone crates/nvs-runtime/src/abi.rs:thrown_as]
