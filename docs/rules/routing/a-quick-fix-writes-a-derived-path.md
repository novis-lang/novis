Nothing derives a `path` from the declaring class. The compiler checks every *internal*
reference to a route and exactly zero *external* ones, so a derivation rule would turn a class
rename into a silent move of the URL that is
in a bookmark, an email, another service's configuration and a search index. A path is a public
interface; a class name is an implementation detail.

What survives of convention routing is the typing, and a quick fix buys it without the coupling: on
the `E0747` diagnostic for a `#[Route]` whose required `path` is missing, the editor offers the
derived path as replacement text the developer accepts, edits or ignores. The derivation is fixed so
two machines produce one string:

- the declaring class's **short name**, a trailing `Controller` removed, `PascalCase` to kebab-case
  (`rule:core-api/identifier-casing`);
- then the method's name, `camelCase` to kebab-case, **omitted entirely for `index`**;
- then one `{name}` segment per parameter carrying no `#[Query]` whose declared type is a capture
  type, in declaration order — a `Request` or a service contributes nothing.

So `UserController::show(uint $id)` offers `/user/{id}`, `UserController::index()` offers `/user`,
and `AdminUserController::listPending(Request $r)` offers `/admin-user/list-pending`. **Nothing is
pluralized**: pluralization is English-only and irregular, and editing the inserted text is the
point of inserting text rather than deriving a path.

This is a fix, not a generator. A generator's output has to be *right*
(`rule:ide/a-code-action-writes-only-what-is-already-determined`), and a URL is not
type-determined by anything; this output is a starting point, read as a diff before it is accepted.
The fix is backed by a diagnostic the checker already emits, since `path` is a required field of
`Core\Route` (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`).

**Built:** no path is derived, and `E0747` reports a `#[Route]` with no `path`
(`crates/nvs-types/src/routes.rs`, `crates/nvs-types/tests/routes.rs`). Its suggestion writes `path:
"…"` before the payload's first field, or as the whole payload where there is none (`derived_path` in
`crates/nvs-types/src/routes.rs`), and
`tests/lsp/actions/a-route-with-no-path-is-offered-the-derived-one.lspt` freezes the edit. The fix is
not safe, since the URL it writes is a starting point.

**Declined:**
- **Convention routing**, a `path` the compiler derives from the class and method. It makes a
  route free to declare, and is pleasant where the URLs are nobody's API, but it welds a public interface
  to an implementation detail. A quick fix writes the derived path once instead, and that path is real
  text: searchable, and stable under every later rename.
