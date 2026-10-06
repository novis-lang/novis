Each entry in a class's `implements` list may carry a `by $field;` suffix. The compiler then
synthesizes, for every method that interface requires, a one-line forward to the named property. The
field must be an ordinary declared property or a promoted constructor parameter, of a non-nullable
class or interface type that itself satisfies the delegated interface; a mismatch is refused where the
clause is written. Two interfaces may delegate to one field, and two fields may each answer a different
interface. A class may still write the member by hand, which is an ordinary override and beats the
forward.

The field is subject to the ordinary initialization rule, or is `lateinit`. A delegated call before
the slot is written throws the error those rules already define — the forward carries that check
itself, because an unwritten slot would otherwise read the forwarding class back and call itself until
the stack is gone.

Three member shapes get no forward and are refused where the clause is written: a `static` member has
no receiver to read the field off, and a variadic or `inout` parameter list is packed and written back
at the call site (`rule:statements/inout-is-written-at-the-call`), so a forward would do that twice.
Writing the member by hand is the way out. Every member the delegation does not supply is owed exactly
as it would be without the clause.

It spends one pointer-sized property per delegated interface per instance, plus the delegate object,
and that state is an ordinary property a reader can inspect.
