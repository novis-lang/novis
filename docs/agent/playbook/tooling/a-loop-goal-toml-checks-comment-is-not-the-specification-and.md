- **A `loop-goal.toml` check's *comment* is not the specification, and it can contradict a settled
  rule.** The toml wins over a plan field, because both are status; it does not win over a rule,
  because only one of those is a decision — a comment describing `==` as component comparison
  contradicts `rule:expressions/object-identity-equality`, under which `==` on objects is identity
  and `compareTo` is the spelling for content equality. One `bun nv peek` of the cited rule before
  writing the member settles it, and the comment is the thing to fix. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
