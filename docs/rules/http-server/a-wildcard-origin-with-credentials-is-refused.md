`origins = ["*"]` together with `credentials = true` is refused — at boot as `E0612` naming the
line, in `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s diagnostic shape,
and at runtime by `Core\Config::set` returning `false` and leaving the value unchanged
(`rule:config/a-refused-set-returns-false`). Refused rather than warned, because the combination
is not risky: it is meaningless. Every browser rejects it, so a deployment that wrote it has an
access-control policy that does not do what it says and no signal that it does not.

`[http.cookies] same_site = "None"` with `secure = false` is refused by the same mechanism for the
same reason — browsers reject the pair, so it is a policy with no meaning rather than a weak one.

The two mechanisms are held to one implementation of each condition: the guard asserts that boot
and `Core\Config::set` *agree* over a table of moves into and out of each pair, rather than
asserting what either answered on its own.
