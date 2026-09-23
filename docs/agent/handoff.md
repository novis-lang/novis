# Handoff

## State

Goal `core-html-and-1-more` is reached. Every `Core\Html` member and `Core\Http::allowUrl` has all
its feature proofs, and `python tools/dossier.py --verify --group 'Core\Html'` and `--group
'Core\Http'` both pass. The `allowUrl` slice fixed one bug: a URL with an empty host
(`http:///x`) was refused as a `net.connect` grant for a host called ``, and is now refused as
naming no host. The proof programs get their `net.connect` grant from three `[[app]]` blocks in
the root `nvs.toml`. Last session's `sanitize` attack was hiding a quadratic parse, which is now
`# Known gaps` 1 in `crates/nvs-stdlib/src/html.rs` (owner M12), and its attack is marked.

## Next group

The driver picks the next goal. This goal owes nothing, and its DONE gates (`verify.py --doc`,
`owners.py --closes`, `playbook.py --closes`) are all green.

## Backlog

- `[::127.0.0.1]` (IPv4-compatible) and `[64:ff9b::7f00:1]` (NAT64) are approved by the address
  table. `rule:security/net-address-policy` names only the IPv4-mapped forms, so denying them is a
  rule change for the user to decide; `crates/nvs-config/src/capability.rs:285` is where it would go.
- Deep HTML nesting is quadratic: `crates/nvs-stdlib/src/html.rs` § *Known gaps* 1, owner M12.
- A refused host is quoted whole in its message, so a one-megabyte host makes a one-megabyte
  error message (`nvs_runtime::capability::pin_host_addresses`). It is bounded by the input.
