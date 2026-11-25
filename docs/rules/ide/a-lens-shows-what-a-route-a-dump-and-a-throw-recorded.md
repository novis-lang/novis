Three more code lenses read the debug index as the query lens does: one above a route handler, one
above a dump and one above a throw, each saying what the recorded requests did there and opening it
in the viewer with a click.

- **Above a `#[Core\Route]` handler:** how many requests the index holds for it, their p50 duration,
  and the newest failing status with its age. It is keyed on the method and the route pattern, so an
  edit to the file does not move it. A click opens the request list filtered to the route.
- **Above a `Core\Debug::dump` call:** the newest value dumped at that file and line, as one line of
  at most 80 characters from the already redacted tree
  (`rule:security/redaction-reaches-the-tools-own-renderings`). A click opens that request's Dumps
  tab.
- **Above a `throw` whose site an exception group names:** how often it was thrown and when last. A
  click opens the group (`rule:testing/an-exception-is-one-row-per-class-and-site`).

**A dump or a throw lens shows only while the file is not newer than the record,** because a line
number means nothing after an edit above it. The route lens has no such bound, because its key is
not a line.

All three are under `nvs.codeLens.enable`, read `debug/index.sqlite` read-only without waiting, and
show nothing when there is no index, as
`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site` says for the query lens. A click
calls `nvs.openDebugViewer` with the path (`rule:ide/the-debug-viewer-opens-from-one-button`), and a
client that did not name that command gets a lens with a title alone.

- **Memory:** in the editor process, a few numbers and one line of at most 80 characters per lens,
  freed with the answer.

**Declined:**
- **Remapping a recorded line through the file's edits.** The editor would keep the history of every
  edit since the record, and a wrong remap puts a value on a line that never made it.
- **A lens on every call that a request reached.** The query lens and these three answer the
  questions a developer asks at a line, and more would bury the code under text.
