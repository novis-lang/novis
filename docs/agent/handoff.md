# Handoff

## State

Goal `core-json-and-6-more` is met. Every `Core\Json`, `Core\Jwe`, `Core\Jwe\Key`, `Core\Jwt`,
`Core\Jwt\KeySet`, `Core\Log`, `Core\Html` and `Core\Mail` member owes nothing, and `Core\Mail`
carries its class card. `Core\Mail::send`'s proofs send through `[mail.shop]` in the root
`nvs.toml`: a loopback block on port 1 with no listener, so every run ends in an `IOError` and
nothing leaves the machine. A refused loopback connection costs about 2 s on Windows, so the attack
declares a 30 s limit. The bench measures the address checks that run before the member connects,
because the sweep has no SMTP server to measure a delivery against.
The known gaps this goal recorded stay where they are: `# Known gaps` 1 and 2 in
`crates/nvs-stdlib/src/html.rs` and 1 in `crates/nvs-stdlib/src/json.rs`, owner M12.

## Next group

**The goal is met**: the driver switches to the next goal, whose own handoff replaces this one.

- [x] **`Core\Mail::send`**: about, examples, hostile, perf, tests and the `Core\Mail` card
      (`rule:testing/feature-proofs`, `rule:core-api/reference-card`).
      `crates/nvs-stdlib/src/mail.rs:1044`

## Backlog

- A delivery through Mailpit (`tests/db/compose.yaml`) would give `Core\Mail::send` a Rust test
  of the whole SMTP exchange; nothing schedules one (`crates/nvs-stdlib/src/mail.rs` module doc).
