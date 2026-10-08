Returns the value of the first part of a DN, with its escapes removed. For
`CN=Lee\, Ann,OU=Staff,DC=example,DC=test` the result is `Lee, Ann`.

Use it to show the name of an entry, such as a person's name or a group's name.

**Good to know:** the result is `tainted`, because the value can come from the server or from user
input. When the first part has several values joined by `+`, the result is the first one.
