Novis is MIT and links some eighty permissive components, every one of which conditions redistribution
on reproducing something — MIT and BSD the copyright line and the license text, Apache-2.0 additionally
its `NOTICE`, Unicode-3.0 its own notice. None is satisfied by an SPDX identifier alone; a list of
names is not attribution.

The notice, `THIRD-PARTY-LICENSES.txt` at the repository root, is **generated** by
`tools/nv/cmd/gen-attribution.ts` from the resolved dependency graph reachable from the package that ships,
normal and build dependencies alike, reading each component's own license file from its source. Four
properties are load-bearing:

- **It fails closed.** An SPDX identifier with no policy, a component whose source is not fetched, a
  chosen license with no text anywhere in the tree, or an identifier missing from `deny.toml`'s allow
  list is an error, never a silently omitted notice. A new license entering the tree is a decision made
  in `PREFERENCE` and in `deny.toml`, and the two must agree exactly.
- **It is host-independent.** The list is not filtered by target, so a Windows and a Linux checkout
  produce identical bytes; over-including is the right direction to err.
- **Texts are deduplicated by content, not by identifier**, because MIT obliges reproducing each
  component's *own* copyright line.
- **A dual license is resolved to one, and the choice is shown.** `PREFERENCE` orders MIT first; an
  `AND` keeps every conjunct.

Dev-dependencies are excluded — they are linked into nothing a user receives. The file is committed and
CI diffs it (`rule:testing/attribution-is-diffed-in-ci`), and the binary carries it
(`rule:packaging/the-notice-is-embedded-in-the-binary`).
