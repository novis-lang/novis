The algorithm comes from the key, never from the token. A token's declared algorithm header is
*checked against* the key's algorithm and rejected on mismatch; it is never consulted to select one.

That removes the "no signature" family and the asymmetric-to-symmetric confusion class in one stroke,
because there is **no code path in which an attacker-supplied string selects a verifier**. It is not a
check that could be forgotten at a call site — the caller does not choose an algorithm, so there is
nothing for the caller to get wrong.

The same shape runs through the roster: a key of the wrong kind is a verdict on the token, while a
value that is not a key at all is a bug in the program, and the two are reported differently
(`rule:security/verification-throws-and-compares-in-constant-time`).
