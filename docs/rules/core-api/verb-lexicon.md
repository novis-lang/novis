One verb means one thing, so a member's name predicts its return type. `is…`, `has…`, `contains` and
`startsWith` answer `bool`; `find…` answers `?T`; `indexOf` and `keyOf` answer `?uint` or `?K`; `count…`
answers `uint`; `to…` and `from…` convert and construct. Two constructor spellings join them: the unit or
component a value is built from (`Duration::seconds`, `TimeOfDay::at`), and `of` for a canonical identifier
(`Zone::of`, `Hash::of`). A **stateful** object — a response, a session, a config overlay — may use `set…`
and `add…`, which is not a mutation of a value and so not a question for
`rule:core-api/nothing-mutates`.

`…OrNull`, `…Safe` and `…Ex` are banned, and so is `try…` on every verb but one. The exception is
`tryParse` (`rule:expressions/try-parse`), for a class whose `parse` takes exactly one `string` and can
fail: a malformed string is a *failure*, not the absence `?T` means, and `as ?T` never targets a class, so
without it `Core\Uri` and `Core\Uuid` would have no non-throwing spelling at all. The `…Safe` half of the
ban reads the suffix as a claim about failure; a suffix that is an ordinary adjective of the subject is
untouched, which is how `Core\Db\Schema::applySafe` is admitted — it is named for the grade of the steps it
will run and is the more refusing of the two, not the quiet one.
