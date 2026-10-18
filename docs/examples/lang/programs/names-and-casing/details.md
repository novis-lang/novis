Each kind of name has its own error code. `E0110` is for a class, an interface, an enum, an enum
case, a type alias or a part of a namespace. Each of these must start with a capital letter. A method name is
`E0111`. A property, a parameter or a variable is `E0112`. A class constant that is not all capitals
and underscores, such as `MaxLines` or `MAX_lines`, is `E0113`. The message shows the name written
the right way.

A class constant may contain an underscore. No other name may: `$line_count` is `E0112`, and an enum
case `GOLD_RANK` is `E0110`. An enum case `GOLD` compiles, because it starts with a capital letter and
has no underscore.

A constructor written `__construct` is `E0114`.

Every property, method and class constant must say `public`, `protected` or `private`. Without one,
the program does not compile (`E0122`).
