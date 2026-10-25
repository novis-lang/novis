- **An acceptance sweep that a merge into `main` lands in can report red where the tree is green.**
  `floor-debt` builds `target/proof/nvs` once, at its start, and runs for an hour. A branch
  fast-forwarded midway changes the `.out` files and rendered data under it, so the old binary fails
  a new `.out`, and `website-render` sees a half-landed render. Before you fix such a red, compare
  its time with `git reflog --date=iso`. If a merge came after the sweep started, rebuild with
  `cargo build --profile proof --bin nvs` and run the red program again. See `tools/nv/proofs/run.ts`.
  `until: gone tools/nv/proofs/run.ts:export const proofBinary`
