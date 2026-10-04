Where an anonymous function appears in a position whose expected type is a callable type, each parameter
the literal does not annotate takes its type from the corresponding position of that type. A
parameter the literal *does* annotate is checked against it under `rule:types/callable-variance`, and
wins where it is wider.

```php
$users;                                       // array<User>
Core\Arr::map($users, fn($u) => $u->name);    // $u : User, U : string ⇒ array<string>
```

A type variable binds from the argument the signature names first, and the substituted parameter type
is then the expected type pushed into the literal. So the common callback gets *more* checking than a
dynamic one while writing *less*: `$u->name` stops being an erased-receiver fetch
(`rule:types/erased-member-access`) and becomes a field proven present.

A block-bodied `fn` still declares its own return type; inferring one under an expected type is
whole-body return-type inference and is not part of this (`rule:types/anonymous-function`).
