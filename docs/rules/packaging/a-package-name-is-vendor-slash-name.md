A package's name is its root namespace, which has exactly two segments, `Vendor\Package` — `Acme\Http`
— and every file in the package declares that namespace or one under it
(`rule:packaging/a-package-file-stays-under-its-declared-prefix`). Nothing else about the namespaces or
class names inside a package is required.

**The command line and URL spelling is derived, never chosen.** Lowercase the namespace and replace `\`
with `/`: `Acme\Http` is `acme/http`. Nobody types a name into a form. Two namespaces that differ only
by case map to one name, and the registry refuses the second at publish. A name only finds a digest;
the digest is the identity (`rule:packaging/a-package-is-its-digest`).

**Who may take a name:**

- The first publish of `Acme\Http` takes the name. Nothing is reserved in advance, and nothing expires.
- **The vendor is owned.** The first publish under `Acme` makes that account the owner of the vendor
  `Acme`. Only the vendor's owners publish further `Acme\...` packages, and an owner may add other
  accounts as owners.
- A name that has ever had a published version is **never reassigned**, by an owner or by the operator
  (`rule:packaging/the-operator-only-withholds`).
- `Core\` and `Novis\` are refused for every account but the project's own. `Core` is the standard
  library's (`rule:core-api/reserved-namespace`). `Novis\` holds the built-in components and the
  first-party packages, so no package takes a built-in component's name such as `Novis\Image`. The
  first-party REST client is `Novis\Rest`, published by the project account and held to every rule
  here, because a registry whose maintainers do not depend on it does not stay good.

Because the name is the namespace, one registry cannot hold two packages that claim one prefix, and for
a registry package the grant line and the package are the same thing
(`rule:security/grants-are-keyed-on-a-namespace`). A name derived from the code cannot drift from it.
Taking a name costs a real publish from a public repository
(`rule:packaging/the-registry-builds-the-archive-from-git`), so there is nothing to squat before code
exists.

**Declined:**
- **A `vendor/name` chosen on a form, beside a declared namespace prefix.** Two names for one package
  can disagree, two packages can claim one prefix, and the grant line would cover a prefix rather than
  a package.
- **Reserving a name before its first publish.** A reservation is a name with no code behind it, which
  is what squatting is.

The client derives the slug from the namespace and refuses `Core` and every built-in component's
namespace (`crates/nvs-cli/src/package/manifest.rs`).

**Not on disk.** The registry, which owns each vendor, refuses a name that differs from a taken one
only by case and checks the `Novis\` account at publish, is not written yet.
