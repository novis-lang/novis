The six tests (`rule:core-api/tier-placement`) put a candidate in exactly one of five places:

| Placement | What it means |
|---|---|
| **Core** | Tier 0. Compiled into every binary, reachable under the `Core` namespace, no build flag, no capability question about its presence |
| **Native** | Tier 2. Statically linked in the default distribution, gated at run time by a capability and removable at build time by a feature flag |
| **Ext** | Tier 1. A sandboxed component, first-party or third-party, distributed through the package channel |
| **Dropped** | Not implemented at any tier, and named with its replacement |
| **Answered** | The problem is removed by the runtime's own architecture, so there is nothing to port |

Distribution and authority are separate decisions and only the second is security-relevant: a driver may
ship behind a feature flag that defaults on while its network capability stays deny-by-default. The two
that look alike are Native and Ext, and the line between them is test 1 — a component that must hold state
across requests cannot be sandboxed, because handing it a host-owned handle is writing it natively with
extra steps.
