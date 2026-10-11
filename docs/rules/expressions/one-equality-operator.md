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

Loose comparison is a hazard family, not a convenience: a falsy value passing a loose test is why
verification returns claims or throws (`rule:security/verification-throws-and-compares-in-constant-time`). Removing the
construct closes the family, and security and request isolation are not traded for familiarity. The
standard library already compares strictly: `Core\Arr::contains` is always strict, and the natural
ordering throws on a pair from two different rows rather than converting one side. One operator keeps
the library and the language on one answer. `==` is also the spelling of every language without
juggling, so a reader arriving from anywhere but PHP already knows it.

The porting cost is real and lands mostly on one row: `"1" == "01"` is true in PHP and false here, and
nothing in the compiled program flags it. A defensive `$x == null` on a non-nullable type, copied from
PHP, stops compiling; the check was always dead.

**Declined:**
- **Both operators, with PHP's meanings.** That brings the loose-comparison hazard family back, and with
  this type system the two spellings would disagree only on the three rows nobody chooses the loose
  reading for.
- **Both operators, with `==` defined as strict.** Two spellings for one operation
  (`rule:statements/nothing-gets-a-second-name`), and the worse one: a PHP reader seeing `==` assumes
  juggling that is not there.
- **`===` kept and `==` dropped.** `===` exists only as an escape from a loose `==`. With nothing to
  escape, it is three characters saying what two say everywhere else.
