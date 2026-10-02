# Decisions only a goal's prose held

This file is a record for reading. It binds no goal, and a later goal may decide differently without
saying why. **A rule fragment under [docs/rules/](../rules/) is the only binding form**: a decision a
later goal must honour is written there, and this file is not where to look for it.

Each entry below is a standing decision that, when goal `goal-closeout` read the goals in front of it,
had no home but that goal's prose: no rule fragment, decision record, module doc, reference page or test
stated it. An entry says what was decided, where in the tree it applies, and the goal it came from. Work a
goal promised and did not build is not here: it is a gap record under `data/gaps/`.

## Decisions

- **`Core\Digest` leaves out table hashes and dead algorithms.** `xxh*`, `murmur3*`, `fnv1*`, `adler32`
  and `joaat` are table hashes, and an enum that also has `Sha512` would invite using one where a strong
  digest is needed. `md2`, `md4`, `ripemd*`, `whirlpool`, `tiger*`, `snefru*`, `gost*` and `haval*` are
  out because nothing current uses them. `sha3` and `crc32c` are in, because each is worth a crate: SHA-3
  rides the same traits as the SHA-2 family, and CRC32C is the checksum object stores send. Applies at
  the `DIGEST` roster in `crates/nvs-stdlib/src/hash.rs` and spec § 11's `Digest`. From `core-depth`.
- **No `Core` member names a qualifier, and the taint diagnosis has no opt-out.** `body`, `post`, `query`,
  `header`, `cookie` and `Part::filename` all return tainted values, and their names do not say so. A
  `taintedPostAs` would make every other member's name read as a claim it does not make. A caller who
  finds the diagnosis verbose writes `tainted {…}` over the shape. Applies at `postAs` and its siblings
  in `crates/nvs-stdlib/src/request.rs`. From `input-shapes`.
- **A guided install that cannot finish ends one way, whatever stopped it.** An unsupported platform, a
  release list with no matching `major.minor` series and a network failure all end the same way. The
  status item says which of the three it was, the release page stays one command away, and the
  extension keeps working as a grammar-only client. Applies at the install commands `rule:ide/the-extension-guides-an-install-and-never-bundles-one`
  designs, over `editors/vscode/src/install.ts`. From `editor-install`.

## Goals with nothing to keep

`concurrency`, `governance`, `core-part-ii`, `database`, `server`, `carried-gaps`, `warm-start`,
`temp-sweep`, `program-id`, `schema`, `typed-callable`, `doc-comments`, `resilient-tree`, `surface`,
`lsp-server`, `editor`, `request-json`, `test-request`, `parses`, `unix-sockets`, `per-core`,
`net-os-signal`, `formats`, `encoder-cycles`, `record-origin`, `agent-surface`, `xml-tree`, `gap-owners`,
`unowned-sweep`, `signed-urls`, `type-test`, `queue-purge`, `sqlite-queue`, `serve-runs-the-queue`,
`workspace-index`, `editor-surfaces`, `resource-ceilings`, `config-is-written`, `event-streams`,
`finish-response`, `markup-literal`, `fmt`.
