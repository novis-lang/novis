Returns the current one-time code for a secret. This is the six-digit code that an authenticator app
on a phone shows for two-factor login.

The server and the phone both keep the same secret. Each of them computes the code from the secret
and the current time, so they get the same six digits without talking to each other. A new code
starts every 30 seconds.

The code is a string, because a code can start with a zero. The secret must be at least 16 bytes
long, and a shorter secret throws a `LogicError`. `Core\Random::bytes(20)` makes a secret of the
recommended length.

Most programs only need `check`, which tests the code a user typed. `code` is useful when your own
program must log in, for example in an automated test.

The examples show a code for a new secret, a secret that is too short, and a test that logs in to an
account with two-factor login.
