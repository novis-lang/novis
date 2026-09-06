Nothing derives a `path` or a `name` from the declaring class. The compiler checks every *internal*
reference to a route and exactly zero *external* ones, so a derivation rule would turn a class
rename into a tidy set of compile errors for the `url()` calls and a silent move of the URL that is
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

This is a fix, not a generator. A generator's output has to be *right*, and a URL is not
type-determined by anything; this output is a starting point, read as a diff before it is accepted.
