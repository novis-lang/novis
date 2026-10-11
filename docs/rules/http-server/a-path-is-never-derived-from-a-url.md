A request may **select** an entry point from a set enumerated before it arrived; it may never **construct** one. No filesystem path is computed from a URL at request time, under any dispatch, on any mount.

This is the rule the rest of the server is built to keep. FastCGI's vulnerability class is not "many entry points" but "a URL-derived path": the `SCRIPT_FILENAME`/`PATH_INFO`/`cgi.fix_pathinfo` family exists because a web server computes a path from the request and hands it to a runtime that trusts it. A design that enumerates its entry points ahead of time is immune at any number of them.

So every path the server can execute is known at boot, printed by `nvs info --config`, and fixed until a reload. That is what makes `rule:http-server/a-mount-table-expands-at-boot`'s wildcards safe: a glob expanded against the disk at boot yields a fixed table, whereas the same glob evaluated per request would be `cgi.fix_pathinfo` with a different spelling. A path carrying a dot-segment or an encoded separator is refused before any mount is selected (`rule:errors/path-component-refusals`).

**Declined:**
- **Rewrite rules evaluated per request.** They are the FastCGI defect with another syntax: a URL segment put into a filesystem path at request time. They would also put a regex on the request path and make the mount table impossible to enumerate. The boot-time scan gives the same one line for every module and keeps the executable set enumerable.
