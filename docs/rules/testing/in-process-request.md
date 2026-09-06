`Core\Test::request(...)` builds a request and runs it through the compiled route table and the real
middleware chain — no socket, no port, microseconds per test. What answers it is the program's own
entry, run as an isolate with the match already on its carrier, never the matched handler, which
nothing may invoke directly. **An in-process request may not be made from inside one**: the entry
answering it is the entry that asked, so a second would answer itself forever, and the refusal is at
the door rather than at a depth ceiling that would report an engine limit instead of the mistake.

The response is a `Core`-owned instance whose two readings are members, so a status is `status()`
and never a property. The synthetic request's parameters arrive **`tainted`**, exactly as a real
request's would, so a handler that forgets to launder fails its test rather than production.

`#[Test(server: true)]` binds a real listener for the cases that genuinely need the wire. The port
is the operating system's and the address is loopback, so two suites on one machine never collide
and no suite serves the program under test to a network. The listener is a **sibling** of the test's
isolate rather than a child — leftover work is read off the test's own task — and it is retired when
the test that asked for it has joined. It serves under the **default** policy rather than the tree's:
reading a deployment's configuration would make the test's subject the deployment.
