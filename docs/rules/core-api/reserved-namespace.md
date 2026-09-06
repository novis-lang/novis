`Core` and everything nested under it is reserved. User code and every extension are refused a `namespace`
declaration or a class declaration that shadows anything there, with a diagnostic naming the collision.
There is no renamed-import case to guard separately, because no alias exists to rename one with
(`rule:statements/nothing-gets-a-second-name`); a plain import whose short name collides is an ordinary
duplicate-import error.

Because the roster is compiled into the binary rather than resolved at run time, a `Core` class's **member
and constant lists are closed while compiling**. A misspelled member or constant is named at the call site,
with the real ones offered, instead of reaching a run-time undefined-function fatal. That is also what lets
the checker seed its signature table from the registry and put a `Core` call through the same arity and
assignability checks a user-declared static call takes, with no second code path.

The array domain class is spelled `Core\Arr` rather than `Core\Array`, because `array` is a type atom
(`rule:types/grammar`) and a class of that name would collide with it exactly where a type is expected.
