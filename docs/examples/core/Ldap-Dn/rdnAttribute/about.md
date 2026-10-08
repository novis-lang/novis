Returns the attribute name of the first part of a DN. For `CN=Ann,OU=Staff,DC=example,DC=test`
the result is `CN`.

Use it to see what kind of entry a DN names, such as `CN` for a person or `OU` for a unit.

**Good to know:** the name is returned as it is written in the DN, so `cn` stays `cn`. When the
first part has several values joined by `+`, the result is the name of the first one.
