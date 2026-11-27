**`#[TestOnly]` makes a declaration test code** in `rule:testing/tests-never-reach-a-build`'s sense.
The attribute resolves nominally to `Core\TestOnly`, has no fields, and may be written on a class, an
interface, an enum or a method. On a method it makes that method test code. On a type it makes every
method of the type test code, the constructor included, and makes the type itself a test-only type. It
is not under `Core\Test\*` beside the testing markers, because it is written in production files, next
to the code it marks, where `#[TestOnly]` reads as a plain statement. A class is test code as a whole
only when its author writes the attribute on it.

**Its sites are the attribute table's.** On a property, a constant, an enum case or a parameter it is
the ordinary wrong-site error. A `#[TestOnly]` on a member of a type that already carries it is
allowed and says nothing.

**A use outside test code does not compile** (`E0889`). A use is everything
`rule:packaging/a-restriction-limits-where-a-declaration-is-used` § *What counts as a use* names: a
call, a method reference, a class-reference call, a `new`, a static property, a class constant, an
enum case and a `Foo::class` reference. On top of those, every type position is a use: a parameter
type, a return type, a property type, a constant type, `is`, `catch`, `class<T>` and a type argument,
in a body or in a signature. A restriction lets allowed code pass a restricted object through typed
signatures. A test-only type has no class in a build, so no production signature may name one. The
message has one shape for both kinds of test code: "`Cache::clearAll` is test code, so only test code
can use it", and its help names `#[TestOnly]` when the target carries it.

**Inside test code** a method with a testing marker, a test-only method and every method of a
test-only type may use test-only code. A test-only type may name another in its signatures, and an
anonymous function written inside test code is test code.

**Production code may not depend on test-only code** (`E1019`), in four places:

- a class or interface that is not test code extends or implements a test-only type, or names one as
  a type argument in its `extends` or `implements`;
- a test-only method of a type that is not test-only overrides or implements a method that is not
  test code, because production calls that method through the parent type;
- a method that is not test code overrides or implements a test-only method, because production could
  then call the name through the subclass;
- a test-only class or method declares a route, a command or another entry point the compiled route
  and command tables name, because a run would then name a handler it has no row for.

A test-only type may implement a production interface, such as a fake `Clock`, because no object of
it exists outside tests.

**A run and a build leave it out.** `nvs run`, a reload and `nvs build` lower no test-only method and
emit no class row, descriptor or layout for a test-only type. `Core\Reflect::forClass` with a
test-only class's name throws the error an unknown class throws, and a dynamic call on a `mixed`
receiver finds no test-only method. `nvs test` lowers all of it, and `nvs check` checks all of it.

**`Core\Program::implementing`, `implementingWith` and `constructors` never list a test-only class,
under any command, `nvs test` included.** A test then sees the module set production runs, and a test
that needs a fake in the list passes it in itself. The filter is
`crates/nvs-types/src/test_code.rs`'s `listed_implementors`, read by `crates/nvs-types/src/program.rs`.

**A package's test-only code is test code wherever it is used**, so an application's tests may use a
framework's test helpers. `rule:testing/test-attribute` drops a package's `#[Test]` rows and not its
test code. A package that adds `#[TestOnly]` to a declaration that was not test code makes a breaking
change (`rule:packaging/only-a-major-may-break-the-api`).

**The editor offers it only inside test code.** Completion decides "inside test code" from the
declaration around the cursor (`crates/nvs-lsp/src/completion.rs`'s `test_code_at`): a test-only type
or `extend` block, or a method the checker calls test code. Outside it, `hidden` leaves out every
member of a test-only type and every test-code method, and the type lists leave out test-only types,
except the qualified list a `use` line takes, since a test file imports its test-only types at the
top. A hover on test-only code adds "Only for tests. `nvs run` and `nvs build` leave it out."
(`crates/nvs-lsp/src/hover.rs`'s `for_tests`). Go to definition, find references and rename work as for
any declaration.

**What it costs:** nothing at run time, and a build is smaller by every test-only declaration. At
compile time, one attribute check per declaration in the walk `build_table` already makes, a set of
test-only types, and one set lookup per recorded use and per type position when the set is not empty.
With no `#[TestOnly]` in the program the pass returns at once. Memory: two sets, O(test-only
declarations), once per compile, and nothing per request.

**Declined:**

- *A `[[restrict]]` entry whose `allow` lists the test namespaces.* Test code is not a place: a test
  may live in any file. A restriction also keeps the code in the build.
- *Sorting a test-only item last in completion, as a restricted one is.* Outside test code it is
  always a compile error, so offering it only invites the error.
- *Test-only properties and constants.* They would change an object's layout and its codec output
  between `nvs test` and `nvs run`, so a test would check a shape production never has.
- *Listing test-only classes in `implementing` under `nvs test` only.* The tests would then run a
  different module set from production.
- *Type positions allowed, as `[[restrict]]` allows them.* A test-only type has no class row in a
  build, and a production signature that names it is a test seam in the public API.
- *A test-only route or command kept in the tables under `nvs test` only.* The tests would then serve
  a route set production does not have, the same split as `implementing`'s.
- *A warning instead of an error.* The code is not in the build, so the use cannot run.
