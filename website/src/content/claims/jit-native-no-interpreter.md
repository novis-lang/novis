---
claim: Every function runs as native machine code — there is no interpreter tier
category: performance
comparedTo: [PHP]
proof: 'The compiler lowers every function through Cranelift to native code; ADR 0002 documents the call ABI every compiled function shares. There is no bytecode dispatch loop anywhere in the runtime.'
tradeoff: 'JIT compilation costs startup work and holds compiled code in memory; Novis accepts both (ADR 0004) and amortizes them across requests with an on-disk artifact cache (ADR 0042).'
draft: true
weight: 20
---

PHP interprets bytecode, with an optional JIT bolted onto the interpreter. Novis has no
interpreter to fall back to: compilation to machine code is the only execution mode, so
the runtime is built around it rather than around dispatch. Hot web code does not warm
up into being fast — it starts there.
