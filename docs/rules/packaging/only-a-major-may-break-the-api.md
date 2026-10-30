A package version is `MAJOR.MINOR.PATCH`, read literally, and from `0.1.0` onward a minor or patch
release never breaks the public API of the release before it. A major may change anything.

- `0.0.x` is experimental. Any shape may change in any release, and nothing is checked.
- From the first release at `0.1.0` or higher the API is stable. The second part goes up for a minor
  (features and fixes), the third for a patch (usually fixes), and the first for a major.
- Additions and widening are allowed in a minor and in a patch alike.

Package versions do not follow `rule:packaging/below-1-0-the-breaking-slot-moves-left`, which is the
toolchain's own scheme.

**A break** is any of these, and an addition that breaks a caller counts:

- removing or renaming anything public;
- a new method on a public interface, or a new abstract method;
- a new enum case, because it breaks an exhaustive `match`;
- a parameter type that narrows, or a return type that widens;
- a new required parameter.

A change the comparison cannot classify is a break, and there is no way to suppress one.

**`nvs` checks it statically, without running any code.** `nvs pkg api` extracts the public API
from the checked program after name resolution, so an alias, an import or a formatting change is not a
change, and `nvs pkg diff` compares two versions and exits non-zero on a break the version step
does not allow. The registry runs the same comparison at publish and refuses the release
(`rule:packaging/publishing-never-overwrites`). The first release at `0.1.0` or higher waits for a
one-time passkey confirmation by an owner on the website, which shows the exact API being frozen and
says that from then on only a major may break it
(`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`).

**The guarantee is about shape only.** Behaviour may change in any release, and the reference says
so. Shape is what a compiler can check without running code, and a promise about behaviour would be
one that nothing holds.

**A maintainer who keeps an old major alive publishes a new package**, `Acme\HttpNext` or `Acme\HttpV2`
as they choose, and may set `[package] supersedes = 'Acme\Http'` so that `nvs pkg outdated`,
`nvs pkg audit` and the package page point at it. Nothing requires this. The key is read by
`crates/nvs-cli/src/package/manifest.rs`, and `nvs registry build` prints it.

The check is what keeps `rule:packaging/resolution-takes-the-highest-minimum` sound: a minimum is never
a ceiling, so a higher version inside a major must always be acceptable, and the registry refuses one
that is not. Across a major, a break that matters is a compile error, because the whole program is
type-checked.

**Declined:**
- **A new name for every breaking release**, `acme/http` → `acme/http2`. It kept resolution sound only
  by convention, and it charged every maintainer a rename where a checked major does the same job.
- **A namespace depth per major**, `Acme\Http\V2`. It writes the version into every `use` line and
  class name, and asks something of package code beyond its name
  (`rule:packaging/a-package-name-is-vendor-slash-name`).
- **Cargo's `0.x` shift**, where `0.MINOR` is the breaking slot. A version read literally needs no table
  to explain it, and `0.0.x` already covers code that promises nothing.
- **A mark that suppresses a break the author calls harmless.** A check its author can switch off is a
  convention again.

`nvs pkg api` extracts a package's public API from the checked program, without running any of it
(`crates/nvs-cli/src/package/api.rs`). `nvs pkg diff` compares two versions' APIs, classifies each
change as breaking or additive, and refuses a breaking change in a minor or a patch
(`crates/nvs-cli/src/package/diff.rs`). `nvs api diff` is a different command: it compares two OpenAPI
documents (`rule:routing/api-diff-fails-a-breaking-change`).
