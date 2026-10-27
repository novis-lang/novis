Above a method, a code lens says "used by N routes, M tests". A click runs exactly the tests it
counted, through the Test Explorer's own filtered run
(`rule:ide/the-test-tree-is-discovered-and-run-through-the-cli`).

**A route counts through the call graph.** A route counts when its handler reaches the method through a
chain of calls the checker resolved, the handler itself as the first link. The chain is a call graph
read off the one reference index (`rule:ide/five-features-are-one-reference-index`). Each occurrence
records the function or method it is written in, so an occurrence that resolved to a method is an edge
from that function to it. The graph is the index read one more way, never a second walk of the
workspace. Reach walks callers backwards from the method and visits each function once, so a recursive
chain ends.

**A test counts through the last coverage run, and through the call graph where there is none.** The
user decided this. When the Coverage run profile has run, its JSON report lists the lines each test
reached (`rule:testing/report-formats`, `rule:testing/coverage-report`), and a test counts when it
reached a line of the method. That count is exact for the run it came from. Where no report exists,
a test counts when its `#[Test]` method reaches the method through the call graph, or when it calls
`Core\Test::request` with a literal method and path that the compiled route table matches to a route
that counts (`rule:testing/in-process-request`), and the lens marks the test count "static". The match
is the server's own matcher, so the lens and the server never disagree about which handler answers a
path. A path built at run time is not matched.

A call the checker did not resolve to one method is not followed: a call through a `callable` value,
and a call by name on a `mixed` receiver. Where either could reach the method, the lens puts "at
least" before a count read off the graph. That is the case when the method is taken as a callable
value anywhere in the workspace, or when a call on a `mixed` receiver names a method of the same name.
A call resolved to a method that the reached method overrides is not followed either, because which
of the two runs is decided at run time; where such a call exists, the count is "at least" too.
A count read off the graph is never too high: everything counted reaches the method. A call written
in a closure counts as a call from the method that writes the closure, which is where a reader looks
for it.

The route count and a static test count are read from the index, so they are current after every edit.
A test count from a report is as old as its run, and the next coverage run refreshes it. The lens is
gated on `nvs.codeLens.enable` with the other lenses.

**The client keeps the reports, one per program.** The VS Code client writes the JSON report of each
program's last coverage run into a directory of its own storage, with every file name made absolute,
and names that directory in `initialize`'s `coverage.dir`, as it names `stubs.dir`. A report applies
to a file one of its tests reached. Where none did, the count is the call graph's and says "static". A
test the index no longer declares is not counted. **A method that no route and no test reaches has no
"used by" lens**, for the reason a hierarchy lens is written only where it has something to say.
**Neither has a `#[Test]` method**: it would count 0 routes and the one test that is itself, and the
Test Explorer's run button beside it already runs it. For the same reason, a `#[Test]` method and the
class that declares it have a reference count only when it is above 0. The runner finds both by the
attribute, so "no references" would be true of every test. A click carries `nvs.runTests` with the
tests as `Class::method`, for a client that names that command.

- **Memory:** one function name per occurrence in the index, held in the editor process for the
  index's life, and the newest report's per-test lines while the server holds it. Each lens request
  allocates its visited set and frees it with the answer. Nothing at compile time for running, and
  nothing at run time.

**Declined:**
- **Counting tests by the call graph alone.** A test that reaches the method only through a `callable`
  or a path built at run time would never count, and a test that names the method on a branch it never
  takes would. The coverage run already records what each test reached.
- **Following a call through a `callable` value or a `mixed` receiver.** Its target is known only at
  run time, so a count through it is a guess. "at least" says where the count may be low.
