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

The project owns that framework permanently: its own compatibility surface, its own security reports
and its own release cadence. That is the largest ongoing cost the project carries, and it is carried
because the alternative is the failure mode above. The `Core` half grows the API surface, which is
the expensive resource the tier tests guard, so each entry there is justified by the test that placed
it and none by convenience. The framework is also the registry's heaviest user, so every rough edge
in the package system is felt first-party, first.

A team arriving from an active-record framework will look for the ORM and the container
(`rule:programs/framework-refusals`). Those frameworks provide them through reflection and run-time
magic; Novis provides the same things while compiling, and cannot provide them any other way without
giving up the guarantees it exists for. That answer belongs on the documentation's first screen.

If the boundary between the halves is crossed repeatedly, by a package member that keeps needing a
privileged escape hatch, the tier tests are applied to that member again; no escape hatch is added.

**Partly on disk.** The `Core` half is built (`rule:programs/framework-core-half`). There is no
`nvs/web` package.

**Declined:**
- **Primitives only: router, session and validation in `Core`, the rest left to the ecosystem.** It
  keeps the API surface minimal, and it is precisely the position that leaves a good language beside an
  empty registry. The ecosystem that would fill it does not exist and has no reason to appear first.
- **The entire framework in the binary.** One download and the best first run, but a framework's
  opinions must move faster than a language's surface, and every framework fix would become a runtime
  release.
- **The entire framework as a package, privileged parts included.** Sessions, validation, queues and
  persistent connections fail the privilege, sink and waiting tests: a package cannot launder a
  qualifier or hold state across requests, so the privileged pieces would end in `Core` anyway, behind
  a worse boundary.
- **A placement rule specific to framework components.** It would be a second authority over the same
  question, and the tier tests answered every case without amendment
  (`rule:core-api/tier-placement`).
