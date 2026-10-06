`Core\Reflect` and `Core\Ast` are built-in `Core` domain classes, present in every Novis program with
nothing to install: read-only structural introspection over a program's own classes, interfaces,
enums, functions, properties, constants, attributes and parameters on one side, and a parser for
Novis source text on the other. Neither is an extension a deployment might lack, because leaving
either to userland produces a split — reflection native and mature, a real syntax tree only from a
third-party parser whose grammar drifts from the engine's.

**There is one parser.** `Core\Ast::parse` calls the same lexer and parser the compiler runs, so a
construct that compiles parses identically at run time, a construct the compiler rejects is rejected
identically, and a linter, a codemod or a formatter is a program any Novis user can write rather than
a privilege of the toolchain's own Rust (`rule:core-classes/ast-is-inert`).

Two invariants keep both safe. A reflective call or write runs the visibility check and the property
observer ordinary code at that site would face, and there is no `setAccessible(true)`
(`rule:security/reflection-enforces-visibility`). A parsed tree is inert typed data with no path back
into execution, because `eval` does not exist. Neither touches the filesystem, the network or another
process, so neither needs a capability grant (`rule:security/reflection-needs-no-capability`).
