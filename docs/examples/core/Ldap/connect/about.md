Opens an LDAP directory, such as Active Directory, under the name an operator gave it.

An `[ldap.<name>]` block in `nvs.toml` has the server's URL, the account to log in with and its
password. `Core\Ldap::connect` opens the block with that name and logs in as that account. Your
program writes the name and nothing else, so a password never reaches your source code.

The block can list more than one URL. The servers are tried in that order, and the first one that
answers is used. If none answers, the call throws a `Core\Ldap\LdapError`, and its `kind` is
`Unavailable`.

Every call with the same name returns the same connection while the request runs. The connection is
closed when the request ends.

**Good to know:** the operator decides which blocks a program may open. `[capabilities.ldap] connect`
in `nvs.toml` lists the names. A name that is not in the list throws a `RuntimeError`.
