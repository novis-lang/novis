Checks if a DN is another DN, or an entry below it. The result is `true` or `false`.

Use it to check that an entry is in the part of the directory you expect, such as
`OU=Staff,DC=example,DC=test`, before you change it.

**Good to know:** names and values are compared without case, so `dc=EXAMPLE` and `DC=example` are
the same part. A DN is within itself.
