Everything about the shape of a response that a deployment rather than a program decides: the
security headers that go out on every answer, whether a cross-origin caller is answered at all,
what every cookie inherits, and how much an error page is willing to say.

With nothing written, all of it is already closed. The secure headers are on, cross-origin calls are
refused until the origins are named, cookies are secure and same-site, and an error page says that
something went wrong and no more than that. What is written here widens or narrows a closed default
rather than switching a protection on.

A program may change almost any of it for the one request it is handling, and the next request
starts from the file again — a handler that wanted a different policy could have written the header
into its own response by hand anyway. The two it may not touch are the key cross-site request
tokens are signed with and the file that key is read from.
