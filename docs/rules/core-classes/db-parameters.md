The params argument is a single `array<mixed>`. A list-keyed array means positional `?` placeholders;
a string-keyed one means `:name`; mixing the two in one call throws `LogicError`. Both are rewritten
for the driver, and a `:name` used twice binds one value once, which positional form cannot express.
The rewriter skips string literals, comments and PostgreSQL's `::` cast and jsonb operators, which is
why `??` escapes a literal question mark.

**One parameter is always one value.** A list bound to a placeholder is a single value — a PostgreSQL
array column, a JSON document — and `Core\Db::inList($values)` is the explicit marker that expands
into a parenthesised placeholder list. Automatic expansion was refused on the explicit-typing rule
rather than on how common array columns are: it would make the SQL *text* depend on a runtime value's
type, so a `mixed` that turned out to be a list would reshape the query instead of failing.

`inList([])` throws. An empty list means "match nothing" inside `IN` and "match everything" inside
`NOT IN`, the rewriter cannot tell which it is in, and picking one silently is worse than making the
caller branch. Expansion changes the statement's arity, so the statement cache keys on it.
