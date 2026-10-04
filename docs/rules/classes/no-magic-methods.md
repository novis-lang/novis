No method name changes what a class does by being present. Each of PHP's magic methods is either
replaced by a declared interface — `__get`/`__set` by `PropertyObserver`
(`rule:classes/property-observer`), `__toString` by `Stringable` (`rule:classes/stringable`), and
ordering's implicit property walk by `Comparable` (`rule:classes/comparable`) — or removed outright:
`__call`/`__callStatic` (`rule:classes/no-call-magic`), `__destruct`
(`rule:classes/no-destructors`), `__clone` and the four serialization hooks
(`rule:classes/two-copy-depths`), `__isset`/`__unset` (`rule:classes/unset-is-refused-on-a-property`),
`__debugInfo` (`rule:classes/no-debug-hook`), `__invoke` (`rule:types/callable-values`) and
`__set_state`, whose reconstruct-from-generated-code use is answered by the closed round trip instead.

Every one of those names is refused where it is *written*: the method-casing rule allows no leading
underscore, so a class cannot declare a hook for the runtime to decline to call. That is stronger than
"never invoked", and it is what makes an absence checkable at all.

`__autoload` needs no decision — PHP removed it, and every class reference resolves statically, so
there is no runtime moment for a loader callback to attach to. What replaces
`spl_autoload_register` is `rule:programs/no-runtime-autoload`.
