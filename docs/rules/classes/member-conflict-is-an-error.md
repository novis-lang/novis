A method name reachable from more than one source — a default from one implemented interface, a
default from another, or a `by`-delegated interface — is a compile error when the class does not
itself declare that method, and the diagnostic names every contributing source.

There is no `insteadof`. The fix is the ordinary override a reader already knows how to write, and it can still reach a
specific source explicitly — `InterfaceName::method()` for a default, or plain property access
`$this->field->method()` for a delegate, since a delegate is a real object.

**Not implemented.** `E_INTERFACE_MEMBER_CONFLICT` has no code allocated and nothing reports it:
`crates/nvs-types/src/conformance.rs` checks that every required member is answered, one member at a
time, but not that two sources answer the same one. A class reaching two defaults for one name
currently resolves to whichever the member walk finds first rather than being refused.
