Checks whether an entry has a value for an attribute. It returns `true` or `false`.

A directory does not send an attribute that has no value. For example, a person without a job title
has no `title` attribute at all. `has("title")` returns `false` for that person, so you can test for
an empty field before you read it.

Upper and lower case are the same in an attribute name, so `has("mail")` and `has("Mail")` return
the same result.

**Good to know:** `has` only sees the attributes the search returned. If the search's `select` list
does not name an attribute, `has` returns `false` for it, even when the entry has a value in the
directory.
