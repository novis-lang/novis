A user method takes a route the way `Core\Router::url` does, and a call that writes the route is checked
the way a link is. `#[Core\Router\Target]` goes on a `callable` parameter, and `#[Core\Router\Params]` on
an `array` parameter of the same method:

```nvs
public static function entry(#[Core\Router\Target] callable $route,
                             #[Core\Router\Params] array $params): string
```

**Where each may be written.** On a parameter of a method, static or instance, or of a constructor.
Anywhere else, a promoted constructor parameter included, either one is `E0874`. A `Target` parameter's
type is `callable`, a `callable(...)` type, or either one nullable. A `Params` parameter's type is
`array` or `array<...>`, nullable or not. A variadic parameter takes neither, because many arguments
fill it. A wrong type or a variadic parameter is `E0899`. A method has at most one of each, and a
`Params` needs a `Target` in the same method and pairs with it, so a second of either and a `Params`
alone are `E1000`. A `Target` may stand alone, for a route with no parameters. Neither takes a payload,
and no `Core` member carries either.

**What a call checks.** At a call whose `Target` argument is a **written** method reference, the call
asks the four questions `routing/link-target-and-params-are-checked` asks of a link, against the
program's finished route table: the method has a `#[Route]` (`E0754`), a `Params` array literal covers
every capture (`E0755`), each of its keys is a capture or a `#[Query]` parameter (`E0759`), and a literal
value is one the capture's conversion and closed set accept (`E0772`). A reference that names no method
is reported as it is at a link. A named argument is read where it was written. A computed `Params` is
not checked. A parameter cannot default to an empty array (`E0451`), so a `Params` the call leaves
out, which only a nullable default allows, is not checked, and neither is a `Target` with no `Params`:
both are checked for their route alone. Each message names the method the call wrote.

**The call stays an ordinary call.** The check records nothing over the call, so the argument is a real
callable value and the method body receives it. A reference passed as a value — a variable, or a
`Target` parameter forwarded to another such method — is not checked while compiling, and is resolved
when it reaches `Core\Router::url` at run time, which throws for a method with no route.

**The reference keeps its visibility and receiver checks.** A link calls nothing, so it skips them. A
`Target` method receives a callable it can call, so a `private` route method is passed only from inside
its own class. A written reference to an instance method needs the receiver
`expressions/method-reference` asks for: inside an instance method of its own class it carries `$this`,
and from anywhere else the route method is `static`, or the link is written with `Core\Router::url`.

**The table is the program's own.** A reference to a class under an `autoload links` root stays
`E0897` (`routing/a-link-reaches-another-module`): that class is not compiled into the program, so no
callable value of it exists. A link to another module is written with `Core\Router::url` directly.

**The marks are read from the signature the call resolves to**, as `#[Core\Path]`'s are. An override
neither inherits nor needs them, and a call through an override with no marks is not checked while
compiling. The visibility check holds on every path, so that loses checking and not safety.

**Cost:** compile time only. One recorded site per call whose `Target` argument is a written reference,
freed once the table check runs, and one optional pair of parameter indexes per signature. Nothing at
run time and nothing per request.

**Declined:**
- **An `of:` argument naming the paired parameter by a string.** It is a new mechanism for a rare
  method that takes two routes, and a string that names a parameter is not checked by renaming it.
- **Skipping the visibility check for a `Target` argument.** The method receives a real callable and
  can call it, so skipping the check would let any class take another class's `private` method and
  call it. Security and request isolation outranks the convenience.
- **A links-root route at a `Target` argument.** It needs a new run-time value that names a route
  without a callable, and the run time does not change for this feature. A link to another module is
  `Core\Router::url`'s.
