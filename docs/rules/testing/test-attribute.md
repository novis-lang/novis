A test is an ordinary `public` method carrying `#[Test]`, on an ordinary class. There is no naming
convention, no base class to extend and no interface to implement: nothing about a test is inferred
from spelling.

The compiler collects every marked method into a table while it checks, keyed by class label, the
way the route table is built — so discovery costs nothing at startup, and five malformed shapes are
compile errors rather than tests that silently never run: two methods of one class sharing a name, a
`static` method, one returning anything but `void`, one that is not `public`, and one with no body.
The runner constructs the class and calls the member, so each of those is one question about what
shape a test method has, asked once. Each row also carries the span of its method's **name** — where every refusal
about that test already points — because that is the one fact about a test nothing downstream can
recompute, and it is what locates a test in a report and in an editor's test tree.

**The table keeps no `#[Test]` or `#[Property]` row for a class whose file lies in a package below
the entry's folder** (`rule:packaging/a-package-file-stays-under-its-declared-prefix` places the
file). A program loads a dependency to call it, not to test it, and a failing or slow test of
somebody else's code would otherwise fail or slow the application's suite, whether the class came
through `require`, `autoload` or a `discover` glob. The rows leave once every class is checked, in
the step that drops an abstract class's tests, so the package's fixtures and hooks stay, and a
subclass in the application runs every test it inherits (`rule:testing/hook-inheritance`). A
package's tests run when its folder, or a file in it, is the entry, because a `package.toml` in the
entry's folder or above it is the application's. The editor places no file in a package, so its
test tree keeps the rows. Cost: one symbol lookup and one file lookup per class that has a row, once
per compile; nothing at run time.

`#[Test]`'s payload is an options bag — `skip`, `at`, `seed`, `db`, `server`, `retries`,
`because`, `timeout` (`rule:testing/a-test-has-a-wall-time-limit`) and `tags`
(`rule:testing/tags-and-shards-select-what-runs`) — every field optional and every field checked at its own type. An option the roster does
not name is refused, one written twice is refused, and `skip: true` is a `bool` where a `string` is
declared.

The attribute is matched **nominally**, by resolved name: `#[Core\Test]` and a `use`d `#[Test]` are
one attribute, and a userland `class Test` is never it. The marker and the assertions are one class,
so a single `use Core\Test;` places both.

**Declined:**
- **A `test` keyword** (`test "a name is trimmed" { ... }`). It reads better, and it costs new
  grammar, a second scoping rule and a construct the formatter, the language server, the resilient
  parser and the converter must each learn, for a readability gain over a mechanism the route table
  already uses.
- **A `test*` naming convention.** The compiler reads no meaning out of spelling
  (`rule:core-api/identifier-casing`), and a method misspelled `tsetFoo` would silently be nothing.
- **An option that runs a dependency's tests from the application.** A package's tests run from the
  package's own folder, where its manifest is the application's. A switch would put back the one
  case this rule removes, for a run any developer gets by naming the folder.
