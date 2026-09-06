A test is an ordinary `public` method carrying `#[Test]`, on an ordinary class. There is no naming
convention, no base class to extend and no interface to implement: nothing about a test is inferred
from spelling.

The compiler collects every marked method into a table while it checks, keyed by class label, the
way the route table is built — so discovery costs nothing at startup, and four malformed shapes are
compile errors rather than tests that silently never run: two methods of one class sharing a name, a
`static` method, one returning anything but `void`, and one that is not `public`. The runner
constructs the class and calls the member, so each of those is one question about what shape a test
method has, asked once.

`#[Test]`'s payload is an options bag — `skip`, `at`, `seed`, `db`, `server`, `retries`,
`because` — every field optional and every field checked at its own type. An option the roster does
not name is refused, one written twice is refused, and `skip: true` is a `bool` where a `string` is
declared.

The attribute is matched **nominally**, by resolved name: `#[Core\Test]` and a `use`d `#[Test]` are
one attribute, and a userland `class Test` is never it. The marker and the assertions are one class,
so a single `use Core\Test;` places both.
