Checks whether a CSRF token belongs to one session. It returns `true` for a token your own program
created for that session under that key, and `false` for everything else.

`Core\Csrf::verify` takes three things: the token the request sent, the same value the token was
created against, and the same 32 byte key. A token that was changed returns `false`. So does a token
for another session, a token created under an older key, and text that is not a token at all. Only
one mistake throws an error, and it is a key that is not 32 bytes long. That is a bug in the
program, not a bad request, so an ordinary check never needs a `try`.

**Good to know:** this is the only way to check a token. No method gives you the token that was
expected. The check always takes the same amount of time, so an attacker who is guessing learns
nothing from how long an answer took.
