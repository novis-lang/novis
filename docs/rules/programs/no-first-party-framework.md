Novis ships no web framework and no `nvs new` scaffold. `Core` holds every primitive a framework needs
that needs runtime privilege, removes a qualifier, or waits on the outside world
(`rule:programs/framework-core-half`). A framework is composition over that surface: a package
anyone may write, fetched, locked and granted under the same rules as every other package.

**The line is the standard-library tier tests, unchanged** (`rule:core-api/tier-placement`). A
capability that fails one of them is `Core`. One that survives all six — controllers, middleware, login
flows, form binding, pagination, mail composition, a migration runner — is an opinion, and opinions are
userland. No placement rule specific to framework components exists, so there is no second authority
over one question.

**Opinions belong to the teams that hold them.** A framework's choices must move faster than a
language's surface, and they differ between teams. Owning one would be a second compatibility surface,
a second stream of security reports and a second release cadence, carried permanently, and a
first-party one crowds out the ones teams write for themselves.

**The guarantees do not depend on who writes the framework.** Only a `Core` member removes a qualifier,
so a userland framework computes over `tainted` data and never declares it safe, and its authority is
what the application grants it (`rule:security/authority-is-the-enclosing-namespace`). There is no view
layer to ship either: inline HTML and the html template (`rule:core-classes/html-template`) are the
language's own, and the HTML sink escapes by default.

**What a new user reads first is the website.** Its examples are complete programs, one per feature, each
run and checked, so a scaffold would be a second copy of them that has to pick the opinions this rule
leaves to packages.

If a framework package keeps needing a privileged escape hatch, the tier tests are applied to that member
again. No escape hatch is added.

**Declined:**
- **A first-party framework, `nvs/web`, beside `Core`.** It gives a new team a working application on
  the first day, at the cost above: a permanent second product whose opinions the project would own. The
  website's examples carry the first day instead.
- **`nvs new`, a scaffold that runs before it is edited.** It needs a framework to scaffold, and it
  repeats what the examples already show.
- **The entire framework in the binary.** One download and the best first run, but every framework fix
  would become a runtime release.
- **A migration runner in `Core`.** Schema convergence is the model `Core` holds
  (`rule:core-classes/schema-converges`), and a versioned migrator's ordering, locking and history are
  a package's to decide.
