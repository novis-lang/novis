Returns the sign counter of the passkey. The counter is a whole number from 0 to 4294967295.

Some devices count each sign-in, and send the count with each sign-in. `Core\Passkey::verifySignIn`
returns the passkey with the new count. Save that count. At the next sign-in, the device must send
a higher count. If it does not, `verifySignIn` throws a `RuntimeError`. A count that does not rise
can mean that somebody copied the passkey.

Many devices do not count, and always send `0`. When the saved count and the new count are both `0`,
the sign-in succeeds, and the counter stays `0`.

**The examples below** read the counter, save the new counter after a sign-in, and sign in with a
device that does not count.
