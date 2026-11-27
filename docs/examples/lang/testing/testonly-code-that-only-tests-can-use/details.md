You write `use Core\TestOnly;` and then `#[TestOnly]` above the declaration. The attribute has no
options.

A use of test-only code outside test code is `E0889`. A use is a call, a `new`, a static property,
a class constant, an enum case or `Foo::class`. A type is a use too. So a parameter type, a return
type, a property type, `is`, `catch` and a type argument may name a test-only type only inside test
code. The message is "`Cache::clearAll` is test code, so only test code can use it".

A test-only type may name another test-only type. An anonymous function written inside test code
is test code too.

`#[TestOnly]` on a property, a constant, an enum case or a parameter does not compile. A test-only
property would give an object a different shape under `nvs test` than under `nvs run`.

Your program cannot depend on test-only code. Each of these is `E1019`:

- A class or interface that is not test-only extends or implements a test-only type.
- A test-only method overrides a method that is not test-only. Your program calls that method
  through the parent type.
- A method that is not test-only overrides a test-only method. It must be `#[TestOnly]` too.
- A test-only class or method declares a route or a command.

A test-only class may implement an interface of your program. A fake `Clock` for your tests is an
example. No object of the fake exists when the program runs.

When the program runs, test-only code does not exist. `Core\Reflect::forClass` with the name of a
test-only class returns `null`, as it does for a class that does not exist. A call on a `mixed`
value does not find a test-only method. `Core\Program::implementing`, `implementingWith` and
`constructors` never list a test-only class.

A package's test-only code is test code in your program too. So your tests can use the test
helpers of a framework.

In the editor, completion offers test-only code only inside test code. A hover on test-only code
says "Only for tests."
