Every subsystem has a recorded placement, including the ones no milestone has scheduled, because an entry
nobody will build is more useful named than pending. A class's tier is read off the roster rather than
re-argued at the point someone wants it, and a subsystem that is deferred says so with its reason.

The roster also records **splits**: a class may be Tier 0 and unconditional while a backend behind it is
Native and feature-gated, which is how the metrics class and the shared cache tier are placed. That split
is the general rule rather than a special case, and it is what keeps the `Core` name unconditional
(`rule:core-api/core-means-always-present`) while the thing it talks to is optional.

The placements the tests produce, beyond the `Core` classes the registry already holds:

- **Core despite a sandbox case.** `Core\Zip` and `Core\Compress` parse hostile bytes (test 5), but an
  archive's real defects are `../` and absolute-path entries, symlink entries and decompression bombs.
  Those are policy, and policy must be non-optional: a sandboxed decoder gets the memory cap and not the
  traversal rules. `Core\Os` is `posix` minus `fork`, and `Core\Signal` is the graceful-shutdown part of
  `pcntl` and nothing else (`rule:security/isolate-shares-nothing`). `Core\Html`'s parse reads hostile
  markup too, and is Core because the sanitizer must parse to filter (`rule:core-classes/html-parsing`).
- **Native, by test 1 or test 3.** `Core\Db` and the shared-cache backend behind `Core\Cache` hold
  connections across requests; `Core\Ldap` holds a bound directory session, and its escaper is in `Core`
  by test 2. `Core\Crypto` (AEAD only, TLS over `rustls`), `Core\Http\Client`, `Core\Net`, `Core\Mail`'s
  transport, `Core\Storage`'s backends and `Core\Process` wait on the outside world, the network ones
  over the runtime's own reactor rather than a second event loop. A MongoDB client would be Native or
  nothing, and is unscheduled: its query surface shares nothing with `Core\Db`'s, so it would be a
  second database API rather than another driver.
- **Ext, first-party: four components, and the whole list.** Image decoding, which carries the EXIF
  reader because the decoder already holds the file; intl, with its CLDR data and a batch-shaped API
  (`rule:core-classes/intl-batch-shape`); PDF generation (`rule:core-classes/pdf-render-has-no-io`); and
  spreadsheets (`rule:core-classes/spreadsheet-has-no-io`). Tier 1 exists for the third-party channel
  (`rule:packaging/a-package-is-its-digest`), not as a queue of first-party work.
- **Ext when demand appears, and not before.** `bz2`, `xsl`, `yaml`, `msgpack`, `cbor` and
  `protobuf` are pure codecs over hostile bytes, so the tier is settled and only the schedule is open.
  Nothing in Novis depends on them. Vendor-C database drivers (`oci8`, `odbc`, `pdo_dblib`,
  `pdo_firebird`) stay Tier 2, built from source by the operator who needs one, and `Core\Db`'s driver
  interface keeps that possible.

**Partly on disk.** Every class the roster places at Tier 0 is registered in
`crates/nvs-stdlib/src/registry.rs`. The tier-boundary guard in `crates/nvs-stdlib/tests/tier_boundary.rs` holds the rule for what is
registered. The four first-party components are on disk: `extensions/image/`, `extensions/intl/`,
`extensions/pdf/` and `extensions/spreadsheet/`. The image component's `info` reads the EXIF
block (`rule:core-classes/image-pixel-model`), and it decodes every frame of an animation in each
animated format of the roster (`rule:core-classes/image-format-roster`).

**Declined**, each named so it is not re-proposed:
- **The procedural `mysqli`, `pgsql` and `sqlite3` APIs.** One database API (`rule:core-classes/db-one-api`).
- **`filter`.** Its input half goes with the superglobals
  (`rule:statements/no-host-populated-variables`), and its sanitizing filters are half-escaping; only
  the validators survive, as `Core\Validate`.
- **`gettext` and `setlocale`.** Both mutate process-wide C state (`rule:core-api/no-ambient-state`).
- **`pcntl`'s `fork`.** Forking a threaded process is a correctness hazard, and `spawn worker` and
  coroutines serve its uses.
- **`phar`.** Replaced by `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
  and closed with its stream wrapper in `rule:security/closed-doors`.
- **`imap`, `calendar`, and `gmp` as such.** `Core\BigInt` replaces `gmp` over `num-bigint`, not the
  LGPL C library (`rule:types/decimal`).
- **`imagick`, `memcached`, `dba`, `tidy`, `amqp` and `kafka`, by test 6.** The image component,
  `Core\Cache`, SQLite, `Core\Html`'s sanitizer and `Core\Queue` already do each job. Repairing markup
  is the wrong direction (`rule:errors/ambiguous-input-refused`), and `Core\Queue`'s transactional
  enqueue is stronger than a broker's (`rule:concurrency/enqueue-commits-with-your-write`).
- **`soap`, `ftp` and `ssh2`, by test 1 and a thin audience.** `Core\Xml` with `Core\Http\Client`
  covers SOAP; `Core\Storage` and the outbound address policy cover FTP; `Core\Process` runs a real
  `ssh` binary, so SSH credential handling is not owned here.
- **`snmp` and `enchant`.** Outside `rule:programs/audience`; metrics come from `Core\Metrics`, and a
  third-party component can carry spelling dictionaries.
- **Ports of what the runtime already is.** `opcache`
  (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`), `xdebug`
  (`rule:testing/debug-probes`), `swoole`, `parallel`, `event`, `pthreads`, `apcu`
  (`rule:concurrency/cross-request-state-is-explicit`) and `mysqlnd`.

