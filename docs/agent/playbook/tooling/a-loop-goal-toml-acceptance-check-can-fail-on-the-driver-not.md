- **A `loop-goal.toml` acceptance check can fail on the *driver*, not the tree, and a failure detail
  that is a bare `True` is that.** The Python driver's `cargo_check` once read `ordered_in`'s boolean
  as a description of what was missing, so a `kind = "command"` check failed exactly when it passed
  and printed `True` as its reason, and successive handoffs wrote it off as a stale build. When a
  failure's detail is not a sentence about your code — `True`, an empty string, a bare number — read
  the branch in `tools/nv/driver/accept.ts` that produced it before touching the tree. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
