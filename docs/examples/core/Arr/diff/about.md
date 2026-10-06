Gives you the entries of the first array that the second array does not have.

The entries you get keep their own keys, and they stay in the order the first array had. Two entries
are the same when their values are identical: the same type and the same content, with no conversion.
The number 3 and the text "3" are different values here.

You can change what is compared. The `on` option takes `Core\SetOn::Values` (the default),
`Core\SetOn::Keys` or `Core\SetOn::Both`. The `by` option is a function that maps the part being
compared, and it changes the comparison only, never the entries you get back. The `comparator` option
replaces the comparison itself.

The examples show three of these: which values a second list is missing, which keys are new, and a
comparison that treats upper and lower case as the same.
