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

`#[Test]`'s payload is an options bag — `skip`, `at`, `seed`, `db`, `server`, `retries`,
`because` — every field optional and every field checked at its own type. An option the roster does
not name is refused, one written twice is refused, and `skip: true` is a `bool` where a `string` is
declared.

The attribute is matched **nominally**, by resolved name: `#[Core\Test]` and a `use`d `#[Test]` are
one attribute, and a userland `class Test` is never it. The marker and the assertions are one class,
so a single `use Core\Test;` places both.
