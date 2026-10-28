`nvs pkg login` stores a publish token, so that `nvs pkg publish` can use it. You make the token on
the registry's website. Every token starts with `nvsreg_`. It can publish one vendor, such as `Acme`,
or one package, such as `Acme\Http`. It expires after one year at most.

The command reads the token from standard input, so your shell history does not show it. It sends the
token to the registry first. If the registry does not accept the token, nothing is stored. Otherwise
the token is stored in a file that only your account can read.

`nvs pkg whoami` sends the stored token to the registry. It prints the account, the scope and the date
when the token expires. It also prints each release of your vendors that is not available yet.

**Good to know:** if you did not publish a release in that list, somebody else has a token for your
vendor. Revoke the token on the website, and retract the version. A token works only with the registry
it was made for, and `--registry` names another registry.
