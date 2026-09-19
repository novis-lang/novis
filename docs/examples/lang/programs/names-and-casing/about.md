Novis checks the spelling of every name you write, so the spelling alone tells a reader what kind of
thing the name is.

Classes, interfaces, enums, enum cases, type aliases and the parts of a namespace start with a
capital letter. Methods, properties, parameters and variables start with a small one. A class
constant is written in capitals with an underscore between the words. Only the first letter is
checked, so an abbreviation inside a name keeps its capitals and `HTTPClient` and `HttpClient` are
equally fine.

A name spelled the wrong way is an error, not a warning, and there is no setting that turns it off.
The message names the spelling to use instead.

**Good to know:** no name may start with `_`, and a constructor is spelled `constructor`. PHP's
`__construct` and its other magic methods do not exist here. Names are case-sensitive everywhere, so
`Core\Str` and `Core\STR` are two different names.
