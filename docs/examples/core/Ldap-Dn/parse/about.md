Reads DN text, such as `OU=Staff,DC=example,DC=test`, and returns a `Core\Ldap\Dn`.

Use it for a DN that you wrote yourself, for example the base of your searches in a configuration
file. The text cannot be `tainted`. To build a DN from user input, use `Dn::of` and `child`.

**Good to know:** spaces around `,`, `+` and `=` are removed. Text that is not a DN throws a
`LogicError`, and its message gives the position of the first wrong character.
