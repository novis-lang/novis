Checks whether a text is an email address, such as `ada@example.com`. The method returns `true` or
`false`.

An address has one `@`. The part before it has at most 64 characters: English letters, digits, single
dots and some symbols such as `+`, `-` and `_`. It does not start or end with a dot. The part after
the `@` is a domain name with at least one dot, as `Core\Validate::isDomain` checks it. The whole
address has at most 254 characters.

A space, two dots in a row, a letter with an accent or a second `@` give `false`. So does an address
without a dot after the `@`, such as `admin@localhost`. `Core\Validate::isEmail` checks only how the
address is written. It does not check whether the address exists or can receive email.

**The examples below** check a few addresses, show the length limit of the part before the `@`, and
check a list of addresses pasted into a form.
