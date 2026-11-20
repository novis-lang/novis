`$match->attribute<T>()` returns the one payload on the matched route's method written under the
attribute declaration `T`, or `null` where none is. It is the payload
`Core\Attributes::get<T>(Class::method)` returns for that method
(`rule:attributes/a-retrieval-matches-the-declaration-it-names`), so a gate reads a per-route option
such as `#[RateLimit(seconds: 60)]` with `$route->attribute<RateLimit>()` and keeps no table of routes
of its own.

The payload is data folded while compiling (`rule:attributes/retrieval-folds-while-checking`). At each
call site the compiler folds `T` once for every method that carries a `#[Route]`, and the call is given
those payloads keyed by each method's `Class::method` label. The matched row's label is read on the
runtime's side and the entry under it is returned, so the label never becomes a value the program holds
and the payload, a shape of constants, has nothing on it to call (`rule:routing/a-match-is-not-invocable`).
A routed method with no payload of the declaration has no entry, and the call returns `null` for it.

The refusals are the retrieval's own, reported at the call:

- **`T` that is not an attribute declaration** is `E1008`. A payload is found by the declaration it was
  written under, so only a declaration can select one.
- **Two payloads of the declaration on one routed method** is `E0728`, naming that `Class::method`. Any route
  may be the one that matched, so every routed method is held to the one-match rule `get<T>` holds its
  target to.
- **A matching payload with a value that has no constant form** is `E0731`, as it is for `get<T>`.

- **Memory:** each match holds one more integer, its row's place in the table. Each call builds one
  array and one payload object per routed method that carries `T`, and frees all of them but the one
  it returns when it returns. A program that never calls `attribute` pays the integer only.

**Declined:**
- **Options keyed by a route's path.** One method may serve two routes, and a route is identified
  by its method, so the method is what carries an option.
- **Returning the handler label, so a program looks the payload up itself.** The label is the one field
  of a row a program could turn into a call (`rule:routing/a-match-is-not-invocable`).
- **A reflection table read at run time.** Retrieval has an answer before the program starts, and a
  compiled unit carries no reflection table (`rule:attributes/retrieval-folds-while-checking`).
