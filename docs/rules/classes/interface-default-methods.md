An `interface` method may carry a body. A `public` one is a default: every implementor gets it for
free, may override it exactly as it overrides an inherited method, and genuinely *is* the interface —
`$impl is I` answers `true` (`rule:types/type-test`), reflectable and checkable at every call site
that asks for the type, which is what a trait never gave.

`$this` inside an interface's own method body is typed as that interface, not the concrete class.
Only members the interface itself declares, or one it `extends` does, are reachable through it. That
boundary is what lets behaviour and state compose without ambiguity: a default may call another member
the interface requires, but it cannot reach whatever the concrete class also happens to declare, which
would make its correctness depend on who used it.

To call one specific interface's default from an overriding method, `InterfaceName::method()` is bound
to `$this`. That is one small addition to what a reader has to know `Identifier::method()` can mean,
alongside `parent::`, `self::` and `static::`.
