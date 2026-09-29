Checks whether a text is a domain name, such as `example.com` or `mail.example.org`. The method
returns `true` or `false`.

A domain name has one or more parts joined by dots. Each part has 1 to 63 characters: English
letters, digits and `-`. A part does not start or end with `-`. The whole name has at most 253
characters.

`localhost` has only one part, and it is a domain name. A name of digits such as `1.2.3.4` is one
too. A dot at the end, two dots in a row, a space or a letter with an accent such as `ü` give
`false`. `Core\Validate::isDomain` checks only how the name is written. It does not check whether
the name exists on the internet.

**The examples below** check a few names, show the two length limits, and check the domain name a
customer types into a settings form.
