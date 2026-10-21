- **A bench that declares `calls 0` for a `Core` call fails once its member's file changes.** A
  `Core` call counts as a call, but benches recorded before that kept `calls 0` while their figure
  stayed current. Changing the member's file makes them owe a figure, and `--record-perf` then
  refuses every one. Stash the change, run `--record-perf --force` over the group, and set each
  declaration to the count that tree measured. See `tools/nv/proofs/perf.ts`.
  `until: gone benches/members/core/Ast-Node/kind.nvs:bench: calls 0`
