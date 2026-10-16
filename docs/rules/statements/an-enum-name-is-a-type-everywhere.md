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

An enum declared in a class or interface body (`rule:enums/class-scoped-enum`) is the same atom,
spelled `Owner::Status` anywhere and `Status` inside its owner, and its case is `Owner::Status::Active`.
Every binding site above takes the qualified form exactly as it takes the bare one:
`Account::Status $s = Account::Status::Active;` outside `Account` declares the same local that
`Status $s = Status::Active;` declares inside one of its methods.
