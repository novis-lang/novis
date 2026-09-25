- **An enum case behind a `mixed` reads *falsy* when its backing integer is `0`; behind its declared
  type it is truthy.** `rule:types/literal-types` spends no representation on hiding the backing
  value and the runtime table dispatches on the tag, so the erased row diverges from what
  `rule:enums/truthiness` decided. Do not assert the erased row as if it were the rule's answer.
  [until: gone docs/rules/enums/truthiness.md:always truthy]
