The safepoint poll bounds Novis code because it sits between calls. A helper whose runtime is O(its input) — a sort, a scan, an encode, a hash over a large value — is one call, and a deadline that can only fire between calls cannot fire inside it.

**A helper whose runtime scales with its input polls the request's deadline flag**, and the flag lives in `Ctx`'s hot cache line — the one `rule:errors/on-limit`'s stack check already loads, so the poll adds a compare against a line that is already resident and no load. The poll is amortised over a batch of iterations, and the batch is chosen so the amortised cost stays under the stack check's own per-call cost; that bound is what the perf guard holds. Reading a flag rather than a clock is the whole of why this is affordable: a clock read amortised over the same batch would cost more than the iteration it protects.

Two constraints on where a poll may go, both about correctness:

- **The poll is supplied by a bounded-loop combinator, not remembered per helper.** A helper adopts the shape and the shape carries the obligation.
- **A poll site must be a point at which abandoning leaves the value consistent.** A sort cannot be abandoned mid-permutation and its array handed back. Where no such point exists, the bound belongs on the *input* instead — which is what `rule:core-classes/regex-two-tiers` already did for patterns, and the precedent generalises rather than being re-argued.
