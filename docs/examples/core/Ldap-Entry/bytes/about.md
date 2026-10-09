Returns the one value of an attribute as bytes, exactly as the server sent it.

Use `bytes` for a binary value, such as `objectGUID`, `objectSid` or a photo in `jpegPhoto`. These
values are not text, so `string` throws a `LogicError` for them. `bytes` also works for a text
attribute. It then returns the UTF-8 bytes of the text.

If the entry has no value for the attribute, the result is `null`. If the attribute has more than
one value, `bytes` throws a `LogicError`. `toArray` returns every value as bytes.

For the common binary attributes there is an easier way. `uuid` reads `objectGUID` as a
`Core\Uuid`, and `sid` reads `objectSid` as a `Core\Ldap\Sid`.

**Good to know:** the result is `tainted`, because anyone who can change the directory can change
these bytes.
