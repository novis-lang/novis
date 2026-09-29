Reads a text such as `30s`, `1h30m` or `7d` and returns the duration it describes. The text uses the
same syntax as a duration you write in your code.

Use it when a length of time arrives as text while the program runs, for example a timeout in a
configuration file or an option on the command line.

The text is one or more pairs of a whole number and a unit. The units are `w`, `d`, `h`, `m`, `s`,
`ms`, `us` and `ns`, and they must go from the largest to the smallest. There are no spaces, no signs
and no fractions. Any other text throws a `ParseError` that says what is wrong. A duration longer than
about 292 years also throws a `ParseError`.
