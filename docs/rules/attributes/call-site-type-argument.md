`Core\Attributes::get<T>` needs `T` supplied at the call site: no argument has a type it could be
inferred from. An explicit `<T, U>` list written between a member name and the `(` of a call is
therefore grammar, and it is reserved to members the compiler owns — these two retrievals,
`Core\Json::decodeAs<User>($body)`, `rule:programs/implementing` and their kind. It **does not open
user-defined generics**: a type variable stays available only to declarations the compiler owns.

The list is genuinely ambiguous with comparison, since `Foo::BAR < X > ($y)` is also two comparisons,
so it is resolved by a checkpointed trial parse. The `<` opens a type-argument list only when
everything up to a matching `>` parses as a type list with no diagnostic **and** the very next token is
`(`; anything else rewinds and stays an expression. The residue is a `SCREAMING_CASE` constant compared
against a class-shaped name and immediately followed by a parenthesized operand — parentheses say the
other thing.
