A method name reachable from more than one source — a default from one implemented interface, a
default from another, or a `by`-delegated interface — is a compile error (`E0859`) when the class does
not itself declare that method, and the diagnostic names every contributing source.

There is no `insteadof`. The fix is the ordinary override a reader already knows how to write, and it can still reach a
specific source explicitly — `InterfaceName::method()` for a default, or plain property access
`$this->field->method()` for a delegate, since a delegate is a real object.

What is one source and not two:

- **A superclass's concrete method is never a source.** `extends` is walked before `implements`, so
  the inherited method answers and the defaults behind it are not a conflict.
- **One default reached through two interfaces is one source**, as in two interfaces that both extend
  the one declaring it, and a default an interface overrides further down its own chain is replaced,
  not a second source.
- **Only the clauses a class writes itself are compared.** A subclass inherits its parent's answer,
  so a conflict is reported once, at the class whose `implements` list holds it, `abstract` or not.

`crates/nvs-types/src/conformance.rs`'s `check_member_conflicts` is the check.
