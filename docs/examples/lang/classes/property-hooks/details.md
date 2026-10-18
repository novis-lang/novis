A `get` hook can also have a body: `get { return $this->a * 3; }`. A `set` hook may leave out its
parameter. The new value is then `$value`, as in `set { $this->a = $value; }`. A `get` hook that
returns another type than the property is the error `E0401`.

A hooked property has no default value. `public int $a = 3 { … }` is a syntax error (`E0101`).
A hook on a promoted constructor parameter is a syntax error too. A `readonly` property cannot have
a hook, which is the error `E0843`. An interface declares no property, so a hook there is `E0254`.

A write from outside the class to a property with only a `get` hook is the error `E0787`. The
class's own methods may write it. The `get` hook then reads the stored value. A write of one array
element through a hooked property is the error `E0478`, in the class's own methods too. This
includes `$this->xs[] = 1` in the constructor.

Inside the property's own hooks, `$this->name` is the stored value. In every other method of the
class, `$this->name` runs the hooks. So `$this->a = $this->a + 1` in another method runs `get` and
then `set`. A compound assignment such as `$c->a += 5` and an increment such as `$c->a++` run both
hooks too. `unset($c->a)` is the error `E0413`, as for every property.

A subclass inherits the hooks. A write in a subclass method runs the parent's `set` hook. A
subclass may declare the property again with its own hooks, and a read then runs the subclass's
`get` hook. `isset($c->a)` runs the `get` hook, and is `false` when it returns `null`.

Two `get` hooks may read each other, such as `get => $this->b` on `$a` and `get => $this->a` on
`$b`. They then call each other until the program stops with a `RecursionError`.
