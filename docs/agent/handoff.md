# Handoff

## State

**Novis has a TLS client, and `Core\Http\Client` fetches `https` through it.**
`crates/nvs-host/src/tls.rs` is the whole of it: `NvsTls::over` takes a connected `NvsTcp`,
completes a handshake on it and hands back a plaintext `Read`/`Write`. It is a *wrapper* and
not a transport — `rustls` reaches the socket only through the two standard traits, so every
wait inside a handshake or a record read is ADR 0115 § 3's park, unchanged and unaware.

**The trust anchors are compiled in, and that module's doc is the decision's only home.**
Mozilla's set via `webpki-roots`, not the host's store: the same binary then trusts the same
certificates everywhere, a distroless image with no `ca-certificates` still verifies, and
nothing reads a file on a path with no request behind it. A private CA is left to the
**operator** through configuration that has not landed; `tls::upgraded` is the seam it plugs
into. A *program* choosing anchors, or turning verification off, has no spelling and will not
get one.

**`ring` is now a shipped dependency and is spent under ADR 0051 § 4**, whose first question is
`yes` — a record layer is where hostile bytes land. The workspace `Cargo.toml`'s comment above
`rustls` carries the second question's answer. `rustls`/`rustls-webpki`, which is where TLS's
historical failures actually live, stay pure Rust.

**`https` in the outbound transport is `parts.tls` plus three lines.** `transport::exchange` is
generic over `Read + Write`, so `http` and `https` share one copy of the framing, the ceiling
and the retry rules. Two decisions are the transport's own and are in its module doc: the
certificate is checked against the **host the launderer approved** and never the pinned
address, and a certificate that does not verify leaves as a `Fault` (settled) while a timeout
or a reset mid-handshake is `Attempt::Failed` (retried).

**`cargo deny check`'s `advisories` leg is red on a yanked `chacha20` that predates this goal**
and arrives through `rand`; `licenses`, `bans` and `sources` are green, including the
`CDLA-Permissive-2.0` that `webpki-roots` needed added to `deny.toml` and to
`gen-attribution.py`'s `PREFERENCE`. See the playbook bullet.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's
gate over a *complete* Part II, which needs §§ 15-19. Those are `Core\Request` and its
neighbours and are goal 6's, so this check cannot pass inside this goal and is not a
regression.

**No `.nvst` case covers the `https` happy path and none can**: conformance runs with no
network and no TLS peer, so the three Rust cases in `tls.rs` carry it. The client case that
used to pin the `https` *refusal* now pins the header-injection refusal instead — the same
claim (the transport decides before it connects) over a rule that still exists.

**Two pack gaps, both still open.** `[context] modules` does not select
`nvs-runtime/src/terminal.rs`. And `[context] playbook` filters to the *item's* anchor paths,
so a group that also writes `.nvst` cases never sees the case-authoring bullets.

## Next group

**`Core\Mail`'s two owed members, both now unblocked by `NvsTls` — over
`crates/nvs-stdlib/src/mail.rs` alone, with `crates/nvs-host/src/tls.rs` read-only beside it.
ADR 0082 § 2 and the module's own doc § *known gaps* specify them.**

- [ ] **`STARTTLS` on the SMTP session.** `Session` holds an `NvsTcp` today; make it hold either
      that or an `NvsTls` and issue `STARTTLS` after the first `EHLO`, re-issuing `EHLO` over the
      upgraded stream because the advertised extensions change.
      `crates/nvs-stdlib/src/mail.rs:604` is the struct, `crates/nvs-stdlib/src/mail.rs:618` is
      `open`, `crates/nvs-stdlib/src/mail.rs:639` is `command`.
- [ ] **`AUTH PLAIN` over it, and never without it.** `endpoint_of` refuses a configured
      `password` today with a sentence naming the gap —
      `crates/nvs-stdlib/src/mail.rs:291` is the function and
      `crates/nvs-stdlib/src/mail.rs:317` the refusal. `DEFAULT_PORT` at
      `crates/nvs-stdlib/src/mail.rs:101` says 25 *because* this class cannot authenticate, so
      587 is part of the same slice. Refuse `AUTH` on a stream that is still plaintext.
- [ ] **The card and the cases.** `SEND_DOC`'s `errors` at
      `crates/nvs-stdlib/src/mail.rs:177` and the `password` sentence at
      `crates/nvs-stdlib/src/mail.rs:234` both describe the refusal that is going away; the
      module tests start at `crates/nvs-stdlib/src/mail.rs:866`.

## Backlog

- `cargo update -p chacha20` — the yanked crate `cargo deny check`'s advisories leg reports,
  unrelated to any goal-4 slice. Owner: `deny.toml`.
- An operator-configured anchor bundle in `nvs.toml`, plugging into `tls::upgraded`. Owner:
  `crates/nvs-host/src/tls.rs`'s module doc.
- Connection pooling in the outbound transport — ADR 0004 priority 3 against 4, measurable.
  Owner: `crates/nvs-stdlib/src/http/transport.rs`'s module doc.
- Reading `[log] target`, which is what stage 7 actually still owes. Owner:
  `docs/implementation-plan.md`.
- `[context] modules` does not select `nvs-runtime/src/terminal.rs`. Owner:
  `docs/agent/loop-goal.toml`.
- `[context] playbook` filters to the item's anchor paths, so case-authoring bullets never
  print for a group that writes cases. Owner: `docs/agent/loop-goal.toml`.
