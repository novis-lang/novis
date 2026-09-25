- **A goal record's acceptance check can fail on the *driver*, not the tree, and a failure detail
  that is a bare `True` is that.** A driver branch that reads a boolean such as `orderedIn`'s as a
  description of what was missing fails a `command` check exactly when it passes and prints `True`
  as its reason, and a handoff then writes it off as a stale build. When a failure's detail is not a
  sentence about your code — `True`, an empty string, a bare number — read the branch in
  `tools/nv/driver/accept.ts` that produced it before touching the tree. [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
