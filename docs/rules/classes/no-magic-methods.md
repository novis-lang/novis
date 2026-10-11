No method name changes what a class does by being present. Each `__`-prefixed magic method is either
replaced by a declared interface — `__get`/`__set` by `PropertyObserver`
(`rule:classes/property-observer`), `__toString` by `Stringable` (`rule:classes/stringable`), and
ordering's implicit property walk by `Comparable` (`rule:classes/comparable`) — or removed outright:
`__call`/`__callStatic` (`rule:classes/no-call-magic`), `__destruct`
(`rule:classes/no-destructors`), `__clone` and the four serialization hooks
(`rule:classes/two-copy-depths`), `__isset`/`__unset` (`rule:classes/unset-is-refused-on-a-property`),
`__debugInfo` (`rule:classes/no-debug-hook`), `__invoke` (`rule:types/callable-values`) and
`__set_state`, whose reconstruct-from-generated-code use is answered by the closed round trip instead,
with no source emitted that would have to be compiled to rebuild the object
(`rule:classes/serialize-is-a-closed-format`).

Every one of these shares one argument: a behaviour triggered by a reserved method name is invisible
in the class declaration, and a checker or an IDE could read it only by reimplementing PHP's dispatch.

Every one of those names is refused where it is *written*: the method-casing rule allows no leading
underscore, so a class cannot declare a hook for the runtime to decline to call. That is stronger than
"never invoked", and it is what makes an absence checkable at all.

`__autoload` needs no decision: every class reference resolves statically, so
there is no runtime moment for a loader callback to attach to. What replaces
`spl_autoload_register` is `rule:programs/no-runtime-autoload`.

**Declined:**
- **Keeping PHP's `var_export` and `__set_state` format for converted code.** It makes generated
  source a data format, which the closed serialize format exists to avoid.
