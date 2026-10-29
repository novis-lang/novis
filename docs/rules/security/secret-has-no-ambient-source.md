Unlike `tainted`, **nothing in Novis grants `secret` ambiently.** The five host accessors are why
`tainted` can attach itself automatically — every one is a named, enumerable place untrusted data
enters (`rule:security/tainted-sources`). There is no equivalent list for secrecy: an environment
read, a session value, a database row and a string literal all look identical to the type checker
whether or not their content happens to be a credential.

`secret` therefore appears only where a developer spells it on a declaration — a parameter, return
type, property or local. A helper that loads an API key is expected to declare its own return type
`secret string`; the language gives no free ride.

**Three members originate the qualifier**, and each is a member whose *contract* is confidentiality,
so there is no reading of its result that is not a credential. Each is a declared return type, not
an ambient grant.

- **`Core\Cli::secret`** reads a password at a terminal prompt with echo disabled and answers
  `secret tainted string`: the person at the keyboard is outside the program.
- **`Core\Env::secret('NAME')`** answers `?secret string`, `null` where nothing set the variable. It
  reads the variable `Core\Env::get` reads, and a program calls it instead of `get` to say that the
  value is a credential. Its name is a sink, as `get`'s is, so a request cannot choose which
  credential it reads. The value is not `tainted`, for `rule:security/tainted-sources`' reason.
- **`Core\Config::secret('name')`** answers `?secret string` for a `[secrets]` value the operator
  wrote, and `null` where no block sets it
  (`rule:config/settings-and-secrets-are-a-programs-own-values`). Its name is a sink too.

One more call originates it, and it is a fact rather than a guess: **`Core\Config::get` with a
secret directive written as its name** answers `?secret string`. The configuration's own table says
which directives are credentials, because the boot reads each one from a file
(`rule:config/a-secret-is-a-file-whose-content-is-the-value`). That is not the name guess declined
below, which reads a name nobody marked.

**Declined:**
- **`Core\Env::get` answering `secret` for a name that looks like a credential** (`*_KEY`,
  `*_PASSWORD`). A guess about a name is the ambient grant this rule refuses, and it would be wrong in
  both directions.
- **`Core\Env::secret` answering `secret tainted string`**, as the prompt does. An outbound header and
  a database password refuse `tainted`, so every credential read from the environment would need
  `Core\Taint::assertTrusted` before it reached the one place it is for.

The cost is stated: `secret` protects only what a developer remembers to annotate. That is a real
coverage gap relative to `tainted`, not an oversight.
