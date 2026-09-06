A JWT without an expiry, or past one, fails verification. There is no flag to disable the check and no
way for a caller to set an unbounded lifetime: a caller wanting a non-expiring credential is not using
JWT for what JWT is.

**This is JWT's rule, not the roster's.** Its reasoning is about what a JWT is for, and the signed
cookie has carried no lifetime since it landed. Where a lifetime is genuinely a parameter — a detached
signature over a payload — it is *written and never omitted*, `null` included, which is a different
answer to a different question and is why the two are not one rule.

Expiry being unrepresentably absent is what makes a leaked token bounded rather than permanent, which
is the failure the historical implementations kept shipping (`rule:security/algorithm-comes-from-the-key`).
