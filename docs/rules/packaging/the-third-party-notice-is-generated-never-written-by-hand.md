Novis is MIT and links some eighty permissive components, every one of which conditions redistribution
on reproducing something — MIT and BSD the copyright line and the license text, Apache-2.0 additionally
its `NOTICE`, Unicode-3.0 its own notice. None is satisfied by an SPDX identifier alone; a list of
names is not attribution.

The notice, `THIRD-PARTY-LICENSES.txt` at the repository root, is **generated** by
`tools/nv/cmd/gen-attribution.ts` from the resolved dependency graph reachable from the package that ships,
normal and build dependencies alike, reading each component's own license file from its source. The
debug viewer's bundle, which the binary embeds, adds the npm packages `viewer/package.json` lists under
`dependencies` and every package they depend on, at the versions `bun.lock` resolves, each license file
read from the installed package. A package used only to build the bundle, such as Babel, ships in
nothing and is not listed. Four properties are load-bearing:

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

A hand-maintained notice is a second copy of a fact that goes stale without telling anyone, and the
lockfile already holds the authoritative graph, so the notice is derived from it. A license outside
`PREFERENCE` stopping the run is meant to be annoying: it is the moment that decision should be made.
Over-including means a reader may find a component their own build does not contain, and the file
says so.

**Declined:**
- **`cargo-about`.** It deduplicates by license identifier rather than by text, which loses each
  component's own copyright line, the part MIT asks for. It would also add an installed tool and a
  template file to CI.
- **A Rust `xtask` crate.** Reading `cargo metadata` needs a JSON parser crate, which adds a dependency
  to the one tool whose job is keeping the dependency set honest.
- **SPDX identifiers only, with a pointer to a website.** MIT, BSD and Apache-2.0 each condition
  redistribution on reproducing text, and a URL is not a reproduction: it is a promise to keep a
  server running.
- **Markdown for the notice.** A `.md` file either fences the license texts, putting stray fence lines
  in the binary's output, or lets a renderer reflow them. Plain text is shown verbatim by GitHub and
  printed verbatim by the binary, from one file.
