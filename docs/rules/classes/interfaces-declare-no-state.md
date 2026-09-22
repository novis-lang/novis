An interface declares no property. Its body is parsed by the class-body grammar, so
`public string $path;` inside one parses, and it is refused where it is written as `E0254` — at the
declaration, the way an enum body refuses a method — rather than by the layout pass, which carries no
property for an interface and would otherwise leave the first read to fail in codegen behind a clean
`nvs check`.

The two things such a property is reached for are already in the interface. A value every implementor
supplies is a bodiless method. A value that is a constant per implementor is a typed constant the
implementor overrides, read as `static::NAME` from a default method, which
`rule:statements/static-is-a-member-modifier` binds to the implementor's class. Shared state is
`implements I by $field;` (`rule:classes/delegation-by-field`), never a slot the interface owns: an
interface names a contract, and its implementors hold the values.
