`Parses` is the interface for a class that is built out of a piece of text.

A class that implements it writes two methods. The first is
`public static function parse(tainted string $s): static`. It returns an object of the class, or
throws an error when the text is not valid. `tainted` says the text came from outside your program,
such as a web request or a command line. The second is `public function format(): tainted string`,
or `format(): string`. It returns the text that `parse` reads, so a link can write the value back
into an address.

Novis needs this interface everywhere text from outside becomes a value: a piece of a route path, a
`#[Core\Query]` parameter, a command argument and a command option. You declare the parameter at your
own class, and Novis calls `parse` for you. A parameter typed at a class that does not implement
`Parses` does not compile, and the error says so.

In a route, `parse` runs before `Core\Router\Gate::allows`. So `allows` can check the object with
`$route->parameter('name')`, and this works the same for a path capture and a query value. If
`parse` throws, the gate's `refused` method gets `Core\Router\Refusal::BadCapture`, and the default
status is `400`.

**Good to know:** the return type `static` means the class the call named, so a subclass that
inherits `parse` gets an object of the subclass back. In a `final` class, `static` is that class.
There, `parse` may return an object that already exists, such as a record it loads by its id.

**Good to know:** `parse` runs for every request that matches the route, before `allows`. It runs
whether the visitor is signed in or not. Sometimes even the time of a lookup must not show whether a
record exists. Then declare the parameter as `string` and load the record after `allows`.

**The examples below** show a product code built from text, what a subclass gets back, a list of
values from a form checked one by one, a class used in a route, and a route that does not compile.
The last one shows a `final` class that loads a stored order, and a gate that gives `404` to a
missing order and to another person's order.
