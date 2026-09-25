- **A member name threaded into a shared reader has to be `&'static str`, because `claim_body` keeps
  it.** Widening `form_of` from `post` alone to `postAs` as well by adding a `member: &str` compiles
  everywhere except `claim_body(ctx, member, …)`, which reports `E0521: borrowed data escapes outside
  of function` at the *call site* and never names the field on `Ctx` that outlives the call. Every
  caller passes a literal, so the repair is `&'static str` down the whole chain — check what a `Ctx`
  setter stores before threading a name through a reader that two members share.
  [until: gone crates/nvs-stdlib/src/request.rs:claim_body]
