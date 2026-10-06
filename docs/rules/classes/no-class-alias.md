A class is reachable exactly where it is declared — under its own short name, in scope through an
import, or by its fully-qualified path — and nowhere else. There is no `class_alias`, and none will be
added later under another name: this is the mechanism the rule removes, not a gap in stdlib coverage.

A second runtime-reachable name for one class is a name a reader, a reflection call and a serialized
payload can each disagree about. Removing it is the same move `rule:statements/nothing-gets-a-second-name`
makes for imports, and it is what makes shadowing a reserved name structurally unreachable rather than
merely refused.

The compile-time synonym that survives is a `type` alias, which names a *shape* and has no runtime
existence at all — and it may not name a single bare class, which would be this rule wearing the type
grammar as a disguise (`rule:types/alias-is-never-a-bare-class`). The cost is that two
libraries choosing one short name means writing the fully-qualified one at the call site.
