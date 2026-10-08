A pooled `Ldap\Connection` is bound as its block's identity for its whole life, and `authenticate` binds on a connection of its own and closes it. No member rebinds a pooled connection, so no request can change whose rights the next request borrows.

The pool follows `rule:security/db-pool-reset-is-a-boundary`: per core, keyed by every URL, the user,
a digest of the password, the TLS mode and CA file, and the configuration generation. An LDAP session
keeps no other state a program can set — a control lives for one operation — so the reset is the
property that the last operation finished: a connection released with a search still paging, or an
operation outstanding, is closed rather than returned.

`authenticate($login, $password)` dials the block's URL list with the same TLS — or, on a connection
`Ldap::open` made, the one address `open` pinned — binds as the login, and closes the connection
whether the bind succeeded or not. An empty login is refused before anything is sent, as an empty
password is (`rule:security/ldap-empty-password-is-refused`). The login may be a DN, `user@upn-suffix`
or `DOMAIN\user`; it accepts `tainted`, being a framed name the server looks up rather than text it
parses. AD's reasons are kinds — `AccountDisabled`, `AccountLocked`, `PasswordExpired`,
`MustChangePassword`, `AccountExpired`, `NotAllowedNow` — and **an unknown user (525) and a wrong
password (52e) are both `InvalidCredentials`**, so a login form cannot tell an attacker which accounts
exist, and every `InvalidCredentials` it throws has one message, since AD's diagnostic would name the
sub-code. The message never carries the password. Each login pays one TCP and TLS handshake.
