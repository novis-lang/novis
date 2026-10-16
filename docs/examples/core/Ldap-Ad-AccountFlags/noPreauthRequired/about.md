Checks if Kerberos gives a ticket for this account without first checking the password. The result
is `true` or `false`.

Kerberos is the system that Windows uses to log users in. Normally it checks that the user knows the
password before it sends a ticket. When this flag is set, anybody can get a ticket for the account.
An attacker can then try to guess the password from the ticket, without contacting the server again.
Keep this flag off unless an old program needs it.

`noPreauthRequired` reads the bit `0x400000` of `userAccountControl`. A normal account with this
flag is `4194816`.
