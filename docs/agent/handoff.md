# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–8 are on disk, and stage 9 is on disk except its end-to-end case.**
[ADR 0180](../decisions/0180.md) is the record and the home of every decision this goal executes.

Stage 9 landed as three pieces. `[http.client.tls]` is a `System`/`Boot` block with `roots`,
`min_version` and `keylog` (`crates/nvs-config/src/tree.rs:531`); each non-`bundled` `roots` entry is
resolved against the file that wrote it and trust-checked there, in the table as well as in the typed
tree (`crates/nvs-config/src/http.rs:341`), which is `[db.<name>] tls_ca_file`'s pass asked of the
process-wide anchors. `E0638`/`E0639`/`E0640` refuse an empty `roots`, a version floor this build
cannot speak, and a `keylog` on a `production` host; `W1009` announces one a `development` host kept.
`nvs_host::tls::configure` builds the process's one `ClientConfig` from a `ClientPolicy` — plain
strings and a path, because nothing in that crate's `src/` reads a configuration file.

**`configure` has no caller yet, and that is the next slice rather than an oversight.** It settles a
`OnceLock`, so a second call is `AlreadyExists` by design: the wiring belongs at the run sites that
own a process, not in `nvs_cli::config::boot_in`, which `nvs check` also reaches through
`config::grants` and which would then build an outbound client for a command that opens no socket —
and create the key log file while doing it.

`roots` is **additive** where the list says so (`["bundled", "corp.pem"]` is both sets) while
`NvsTls::over_bundle` still **replaces** for the one endpoint that names it. The two readings are
deliberate and argued at `crates/nvs-host/src/tls.rs:73`.

Nothing is blocked.

## Next group

**Stage 9's tail and stage 10's opening** — one file set: `crates/nvs-cli/src/main.rs`,
`crates/nvs-cli/src/config.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **`nvs run`, `nvs serve` and `nvs test` install the outbound TLS client before the program
      starts** — read `[http.client.tls]` off the snapshot at `crates/nvs-cli/src/main.rs:1896`, where
      `ctx.set_config` already runs, and at `crates/nvs-cli/src/main.rs:2253`, and hand it to
      `nvs_host::tls::configure` (`crates/nvs-host/src/tls.rs:497`). Not in
      `crates/nvs-cli/src/config.rs:331`: `config::grants` reaches that for `nvs check`, and a second
      call is `AlreadyExists`. A refusal is the boot's, with the file named.
- [ ] **The client's first end-to-end `https` case** — a loopback origin with an `rcgen` certificate
      under a `roots` file, reached through `Core\Http\Client` rather than through `NvsTls` alone.
      That is the seam between `crates/nvs-stdlib/src/http/transport.rs:264` and
      `crates/nvs-host/src/tls.rs` that nothing covers. The two names the check wants are
      `https_call_through_the_client_reaches_a_loopback_origin_under_a_roots_file` and
      `https_call_to_an_origin_no_root_vouches_for_throws_and_is_not_retried`, `-p nvs-stdlib`.
      `rcgen` is already a dev-dependency of that crate.
- [ ] **Stage 10's six host grants** — `tls.anchors`, `tls.pin`, `tls.any_name`, `tls.insecure`,
      `net.connect_to` and `net.downgrade`, each a host list with no `true` spelling
      (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`), joining `Capabilities` at
      `crates/nvs-config/src/tree.rs:249` beside the `[http.client.tls]` block this session added at
      `crates/nvs-config/src/tree.rs:531`.

## Backlog

- `nvs-server`'s own boot path, if it has one separate from `nvs-cli`'s — the TLS install has to
  reach it too (`crates/nvs-cli/src/main.rs:1034` delegates to `serve::run`).
- The REST package and OAuth, `Link`/`Retry-After` parsing and RFC 9457 details are the package's,
  not this goal's — `docs/agent/loop-goal.md` § *Standing decisions*.
