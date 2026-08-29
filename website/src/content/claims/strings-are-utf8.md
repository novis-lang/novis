---
claim: Strings are real Unicode text, and bytes are honestly bytes
category: simplicity
comparedTo: [PHP, Node.js]
proof: 'ADR 0009: `string` is guaranteed-valid UTF-8 indexed by grapheme clusters, `bytes` is a separate type, and conversion between them is explicit and can honestly fail. No Core member takes an encoding argument, and there is no `mb_` twin of anything.'
tradeoff: 'Code that treated strings as byte buffers must say so by using `bytes`, and grapheme-aware indexing costs more than byte indexing — correctness is the default, raw speed is the opt-in.'
draft: true
weight: 20
---

PHP's strings are byte arrays with a parallel universe of `mb_*` functions; forgetting
the `mb_` prefix corrupts text silently. Novis makes the distinction a type: `string` is
always valid UTF-8 and `Core\Str::reverse` will never cut an emoji in half, while binary
data lives in `bytes` with its own functions that never pretend to know an encoding.
