An assignment to `$this`, plain (`$this = …`) or compound (`$this .= …`, `$this ??= …`), is `E0875` at
the target. `$this` is the object the method was called on. A write to it could only rebind the local
name for the rest of the body, which leaves that object unchanged while every later `$this->…` line
reads as if it changed. No program needs the rebinding: a second object is a new local, declared like
any other (`rule:types/declaration`).
