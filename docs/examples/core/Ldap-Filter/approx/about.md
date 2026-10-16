Builds a filter that finds the directory entries where a value of an attribute is close to this value.

The server decides what "close" means. Many servers compare how names sound, so `Smyth` can find
`Smith`. Some servers treat it the same as `Core\Ldap\Filter::equals`. Test it against your own server
before you depend on it.

The value is sent as data, so it may come from a form. A name with a character that an attribute name
cannot have throws a `LogicError`.
