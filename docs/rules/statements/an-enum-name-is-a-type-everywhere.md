The `enum` keyword appears only at the *declaration*. Everywhere a type is used, an enum is spelled with
its own name, exactly like a class:

```nvs
enum Status { Active, Banned }

class Account {
    public Status $status = Status::Active;                // property
    public const Status DEFAULT_STATUS = Status::Active;   // class constant
    public static function ban(Status $s): Status { ... }  // parameter and return
}

Status $s = Status::Active;                                // local
foreach ($accounts as Status $status => $account) { ... }  // foreach binding
```

An enum name is an atom in the type grammar beside a class name — lexically identical to a class reference,
distinguished by what the name resolves to, the way `self`, `static` and `parent` are already contextual
there. Every binding site that requires a type takes one, with no position that admits a class name and
refuses an enum's.
