# Decisions only a goal's prose held

This file is a record for reading. It binds no goal, and a later goal may decide differently without
saying why. **A rule fragment under [docs/rules/](../rules/) is the only binding form**: a decision a
later goal must honour is written there, and this file is not where to look for it.

Each entry below is a standing decision that had no home but a finished goal's prose when that goal was
deleted: no rule fragment, decision record, module doc, reference page or test stated it. An entry says what was decided, where in the tree it applies, and the goal it came from. Work a
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
- **A description that fails the shape check is rewritten, and `about` is never skipped.** Too long
  means it explains edge cases the tests own, too short means the lead sentence carries the page, so
  neither is padded or trimmed to fit. Every feature has a website page, so `data/proofs/policy.json`
  carries no `about` skip, although `tools/nv/proofs/collect.ts` would honour one. From
  `the-description-is-owed`.
- **A limit handler that cannot run without weakening the bound does not run.** Stopping the request is
  security and its report is not, so the bound is left alone and the finding recorded. Applies at
  `Ctx::run_limit_handler` in `crates/nvs-runtime/src/ctx/hooks.rs` and `rule:errors/on-limit`'s
  reserves. From `limit-handler-reach`.
- **`data/proofs/help-backlog.json` only shrinks.** Nothing is added to it: a feature that lands owes
  its help at once. Applies at the `HELP_BACKLOG` reader in `tools/nv/proofs/collect.ts`. From
  `core-class-cards`.
- **`var` is not a house style.** Only the features a goal names are rewritten to `var`, in a
  `foreach` binding or over an array literal. Every other program keeps its written type, because a
  written element type tells a beginner what the loop or the array has, and neither form is preferred
  in a new program. Applies under `docs/examples/`. From `foreach-var` and `var-array-literal`.
- **CI's billing and runners are the user's, and a hosted-runner failure is fixed from its log.** A
  session that finds CI blocked by billing says so and stops, because whether the repository becomes
  public or gets a self-hosted runner is not its call. A failure that does not reproduce locally is
  still a failure, and a platform `cfg` that hides the symptom is not a fix. Applies at
  `.github/workflows/`. From `ci-green`.

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
`tooling-overhaul`, `core-math-1-3`, `core-math-2-3`, `core-math-3-3`, `core-metrics-and-4-more`, `core-net-listener-and-1-more`,
`core-objectmap`, `core-objectset`, `core-os-and-2-more`, `core-path`, `core-process-and-4-more`,
`core-queue-stats`, `core-random-and-1-more`, `core-ratelimit-and-3-more`,
`core-reflect-classinfo-and-1-more`, `core-reflect-enuminfo-and-3-more`, `core-regex-and-1-more`,
`core-request-1-2`, `core-request-2-2`, `core-request-mount-and-1-more`, `core-response-and-1-more`,
`core-router-and-4-more`, `core-serialize-and-1-more`, `core-session`, `core-signal-and-4-more`,
`core-sse-and-2-more`, `core-str-1-3`, `core-str-2-3`, `core-str-3-3`, `core-taint-and-1-more`,
`core-test-1-2`, `core-test-2-2`, `core-test-response-and-1-more`, `core-time-and-1-more`,
`core-time-datetime`, `core-time-duration-1-2`, `core-time-duration-2-2`, `core-time-instant-and-1-more`,
`core-time-zone-and-2-more`, `core-uri-1-2`, `core-uri-2-2`, `core-uuid`, `core-validate-and-1-more`,
`core-xml-node-and-1-more`, `core-xml-writer`, `core-zip`, `tools-cli`, `tools-config`,
`tools-php-differences`, `tools-agents`, `tools-install`, `tools-server`, `plain-comments`,
`path-literals-follow-up`, `program-enumeration`, `coalesce-assign`, `completion-files`.
