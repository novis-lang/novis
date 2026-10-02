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
- **`Core\BigDecimal` is not scheduled.** ADR 0054 § 6 named it, and no spec row and no ratchet key
  asks for it, so it is a feature the user has not requested and not a gap. `Core\BigInt` is the
  arbitrary-precision type, and `decimal` with `Core\Decimal` is the bounded one. Applies at
  `rule:types/decimal`'s "what lies beyond" and the `E0456` card in `crates/nvs-diagnostics/src/lib.rs`.
  From `bigint`.

## Goals with nothing to keep

`concurrency`, `governance`, `core-part-ii`, `database`, `server`, `carried-gaps`, `warm-start`,
`temp-sweep`, `program-id`, `schema`, `typed-callable`, `doc-comments`, `resilient-tree`, `surface`,
`lsp-server`, `editor`, `request-json`, `test-request`, `parses`, `unix-sockets`, `per-core`,
`net-os-signal`, `formats`, `encoder-cycles`, `record-origin`, `agent-surface`, `xml-tree`, `gap-owners`,
`unowned-sweep`, `signed-urls`, `type-test`, `queue-purge`, `sqlite-queue`, `serve-runs-the-queue`,
`workspace-index`, `editor-surfaces`, `resource-ceilings`, `config-is-written`, `event-streams`,
`finish-response`, `markup-literal`, `fmt`, `template-format`, `webcrypto`, `http-client`,
`process-cache`, `outbound-proxy`, `websocket-client`, `plan-truth`, `gap-register`, `m4-refusals`,
`m5-proofs`, `m4b-editor`, `m7-server-surface`, `m8-db-queue`, `m8-stdlib-depth`, `unowned-closures`,
`class-scoped-types`, `worker-placement`, `core-class-tests`, `tds-bytes`, `cache-shared-dial`,
`decided-closures`, `one-type-test`, `test-doubles`, `gap-zero`, `dossier`, `config-directives-1-3`,
`config-directives-2-3`, `config-directives-3-3`, `types-enum-1-2`, `types-enum-2-2`, `types-exception`,
`types-interface`, `lang-programs`, `lang-types`, `lang-expressions`, `lang-statements`, `lang-classes`,
`lang-enums`, `lang-iteration`, `lang-errors`, `lang-concurrency`, `lang-attributes`, `lang-testing`,
`core-arr-1-4`, `core-arr-2-4`, `core-arr-3-4`, `core-arr-4-4`, `core-ast-and-2-more`, `core-bigint-1-2`,
`core-bigint-2-2`, `core-budget`, `core-bytes`, `core-cache-and-5-more`, `core-cli-and-2-more`,
`core-cli-progress-and-6-more`, `core-config`, `core-crypto-and-2-more`, `core-csrf-and-3-more`,
`core-db-connection-and-2-more`, `core-db-row`, `core-db-rows-and-1-more`, `core-db-transaction-and-1-more`,
`core-debug`, `core-decimal`, `core-encoding`, `core-env-and-4-more`, `core-html-and-1-more`,
`core-http-client-and-2-more`, `core-http-response-and-1-more`, `core-http-socket-and-1-more`,
`core-http-stream`, `core-io-1-2`, `core-io-2-2`, `core-io-file-and-1-more`, `core-json-and-6-more`,
`core-math-1-3`, `core-math-2-3`, `core-math-3-3`, `core-metrics-and-4-more`, `core-net-listener-and-1-more`,
`core-objectmap`, `core-objectset`, `core-os-and-2-more`, `core-path`, `core-process-and-4-more`,
`core-queue-stats`, `core-random-and-1-more`, `core-ratelimit-and-3-more`,
`core-reflect-classinfo-and-1-more`, `core-reflect-enuminfo-and-3-more`.
