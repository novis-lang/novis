The kind of run your code is in. There are seven kinds.

`Core\Env::Context` is declared in the class `Core\Env`. The method `context` returns it.

The cases are `Request`, `Command`, `Isolate`, `Schedule`, `Job`, `Test` and `Connection`. The page
for `Core\Env::context` says what each case means.

A `match` over the kind of run needs one arm for each case. Compare with `==` when only one case
matters, for example `Core\Env::context() == Core\Env::Context::Request`.
