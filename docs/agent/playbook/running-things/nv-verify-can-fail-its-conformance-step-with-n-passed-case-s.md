- **`nv verify` can fail its conformance step with "N passed case(s) left no recording" when every
  case passed.** The coverage pass lists the `.profraw` files of a batch before all of them are on
  disk, and its own report then says the second listing found every one. Read the `coverage lost:`
  block in `.agent-tmp/verify-conformance.log`: if it says "the listing at extraction time missed
  files that were there", run verify once more before you touch a case. See `tools/nv/cmd/verify.ts`.
  `until: gone tools/nv/cmd/verify.ts:UNRECORDED_RE`
