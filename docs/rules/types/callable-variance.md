A callable type's **parameters are contravariant** and its **return type covariant**:

```nvs
callable(User, string): string   $slot;

fn (User $u, string $k): string => …    // exact           — accepted
fn (mixed $u, mixed $k): string => …    // wider params    — accepted
fn (User $u, string $k): never  => …    // narrower return — accepted
fn (Admin $a, string $k): string => …   // narrower param  — refused
fn (User $u, string $k): mixed  => …    // wider return    — refused
```

The two refusals are the unsound directions: the slot may be handed any `User`, and its caller was
promised a `string`.

This does not contradict `array<T>`'s invariance (`rule:types/arrays`), and the reason is the one that
section argues from. An array widening costs an O(n) restamp that must not hide inside an assignment;
converting a callable costs **nothing** — no restamp, no copy, no runtime work at all, because the
check is discharged entirely in the checker. Same shape of question, different cost, so a different
answer. What it buys is that a helper written once as `fn (mixed $v): string` stays assignable
everywhere, which is the whole reason such a helper is written.
