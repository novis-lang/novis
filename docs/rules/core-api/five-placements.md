The six tests (`rule:core-api/tier-placement`) put a candidate in exactly one of five places:

| Placement | What it means |
|---|---|
| **Core** | Tier 0. Compiled into every binary, reachable under the `Core` namespace, no build flag, no capability question about its presence |
| **Native** | Tier 2. Statically linked in the default distribution, gated at run time by a capability and removable at build time by a feature flag |
| **Ext** | Tier 1. A sandboxed `.nvsx` component, first-party or third-party |
| **Dropped** | Not implemented at any tier, and named with its replacement |
| **Answered** | The problem is removed by the runtime's own architecture, so there is nothing to port |

The three tiers that hold code each exist because each is the right answer for a different class of
code:

- **Tier 0, built-in.** `nvs-stdlib`, compiled into the binary: native speed, direct heap access, no
  boundary. Fine-grained primitives go here — string, array and arithmetic operations, anything whose
  whole cost is comparable to a call — each a `static` member of a `Core` domain class
  (`rule:classes/no-free-functions-or-constants`).
- **Tier 1, a sandboxed component.** The default path for third-party code, and where hostile-bytes
  parsers go (`rule:packaging/an-extension-is-a-sandboxed-wasm-component`). The first-party image, intl
  and PDF components are built into the binary and always present under `Novis\`
  (`rule:packaging/the-first-party-components-are-built-in`); every other component loads from an
  `[[extension]]` entry. A call across the boundary costs a fixed overhead, so what lands here has a
  coarse API (`rule:packaging/the-boundary-is-the-cost`).
- **Tier 2, statically linked native.** A Rust crate compiled into the `nvs` binary, for first-party
  subsystems that need raw sockets, TLS termination or the heap: the database drivers, the regex
  engine, crypto. It is safe because it is safe Rust, and it is built from source, which is the right
  friction for code that runs unsandboxed.

What the tiers spend is two host interfaces to keep in step: the internal one Tier 0 uses and the WIT
world Tier 1 uses.

Distribution and authority are separate decisions and only the second is security-relevant: a driver may
ship behind a feature flag that defaults on while its network capability stays deny-by-default. The two
that look alike are Native and Ext, and the line between them is test 1 — a component that must hold state
across requests cannot be sandboxed, because handing it a host-owned handle is writing it natively with
extra steps.

The tests decide the partition, and the resulting roster is `rule:core-api/tier-roster`. No other
runtime's packaging is a reason to place a candidate.
