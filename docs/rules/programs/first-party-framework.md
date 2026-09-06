Novis ships a working web framework. A good language beside an empty registry is the position every
language that lost this fight occupied, and a language arriving without a framework asks a team to
build one as their first project — a cost no team accepts for a runtime they have not yet trusted.

It is split in two, and **no new placement rule decides the split**: the standard-library tier tests
answer it unchanged, so a framework capability is placed by the same ordered questions a stdlib
candidate is, and there is no second authority over one question. Anything needing runtime privilege,
laundering a qualifier, or waiting on the outside world is `rule:programs/framework-core-half`.
Everything that survives all six tests — pure composition over privileged primitives — is
`rule:programs/framework-web-package`.

The line exists because the two halves have genuinely different clock speeds. A language's surface must
be stable for years; a framework's opinions need to move. Putting the whole framework in the binary
would lock its cadence to the runtime's, and putting all of it in a package would push privileged
operations across a boundary they cannot cross.
