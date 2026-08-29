---
claim: PHP's most productive remote-code-execution class does not exist here
category: security
comparedTo: [PHP]
proof: 'The spec''s `Core\Serialize` section: no `__wakeup`, no `__destruct`, no `__toString` hooks exist (ADR 0028), so there is no gadget machinery — and `Serialize::decode` refuses tainted bytes outright.'
tradeoff: 'No magic methods also means legitimate magic-method patterns (lazy hydration hooks, implicit string conversion) must be written explicitly.'
draft: true
weight: 30
---

`unserialize()` on attacker-controlled bytes has been PHP's most reliable path to remote
code execution for a decade. Novis removes the entire class structurally: the magic
methods that gadget chains are built from do not exist in the language, and the
deserializer refuses bytes that arrived from outside the process. What would remain —
type confusion through a forged object graph — is closed by the same taint qualifier
that guards every other sink.
