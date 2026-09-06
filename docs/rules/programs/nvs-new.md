`nvs new <name>` writes an application that **runs before it is edited**: a manifest depending on
`nvs/web` alone with its generated lockfile, one `#[Route]` controller returning a view, one
`#[Route]` returning JSON from a derived codec, a session-backed login, a `Core\Db` connection reading
from a sample schema, an example configuration file an operator would edit, and one `#[Test]` that
passes.

Two constraints bind the template:

- **It demonstrates a qualifier doing its job.** The first code a new user reads shows `tainted` input
  laundered by `Core\Validate` before it reaches a sink, and deleting that call makes the scaffold fail
  to compile — which is what makes the demonstration real rather than decorative. A scaffold that leads
  with routing looks like every other framework.
- **It fetches nothing but `nvs/web`.** A scaffold whose first act is to resolve thirty transitive
  packages teaches exactly the habit per-package capabilities exist to discourage, and a fresh
  scaffold's lockfile names one package and nothing else.

The scaffold is a load-bearing artifact. It is the first experience, so it is held to the compiler's
own standard: tested on Windows, Linux and macOS as a first-class CI job, and a change that breaks it
is a regression.
