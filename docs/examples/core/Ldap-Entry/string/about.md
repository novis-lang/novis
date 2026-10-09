Returns the one value of an attribute as text, such as a person's `mail` or `department`.

If the entry has no value for the attribute, the result is `null`. A directory does not send an
attribute that has no value, so `??` is a simple way to give a default.

`string` reads an attribute with one value. Some attributes have several values, such as a person's
`memberOf`. For those, `string` throws a `LogicError`, and `strings` returns every value. A binary
value, such as `objectGUID`, is not text. For it, `string` throws a `LogicError`, and `bytes`
returns it.

**Good to know:** the result is `tainted`. Anyone who can change the directory can change this
text, so escape it before you write it into a web page, for example with `Core\Html::escape`.
