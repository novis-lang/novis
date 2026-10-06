There is **one** equality operator, `==`, and its negation `!=`. `===` and `!==` do not parse
(`E0232`); `<>` does not parse either (`E0241`), and each diagnostic names the edit. There is no
suppression and no dialect flag: a construct that parses in one project and not another is two
languages.

`!=` is exactly `!( … == … )`. There is no third relation.

Nothing for a second spelling to distinguish survives, because there is no loose comparison to escape
from. `==` never converts either operand; both operands must be able to hold the same value
(`rule:expressions/disjoint-comparison-refused`); each type's answer is the strict one
(`rule:expressions/equality-semantics`), objects compare by identity
(`rule:expressions/object-identity-equality`), and only a `mixed` or union pairing is decided at run
time (`rule:expressions/mixed-equality`).

Ordering — `<`, `<=`, `>`, `>=`, `<=>` — is a separate question with its own rule, and this operator
says nothing about it.
