---
id: attributes
title: Attributes, routes, commands and derived codecs
summary: `#[...]` metadata as shape literals, how it is read back, and the names the compiler acts on — JSON codecs, the route table, the command table, and program enumeration
keywords: attribute, #[...], Attribute, Reflection, ReflectionAttribute, getAttributes, Core\Attributes, get, all, #[Route], Symfony route, Laravel route, Route, Access, Query, Api, OpenAPI, Router::url, urlAbsolute, Command, Option, Symfony Console, Json\Derive, Json\Field, JsonSerializable, decodeAs, Program::implementing, autoload
---

# An attribute is a shape literal attached to a declaration

`#[...]` in front of a declaration attaches metadata to it. The payload is a shape literal, and the
name in front of it is a `type` alias of a shape that the literal is checked against — an attribute
is never a class, is never instantiated, and has no behaviour of its own.

```nvs
<?nvs
type Column = {name: string, unique: bool};
type Route = {path: string};
type Audited = {};

#[Route(path: "/users")]
#[Audited]
class Users {
    #[Column(name: "user_name", unique: true)]
    public string $name = "";

    #[Route(path: "/users/{id}")]
    public function show(#[{inject: "repo"}] int $id): int {
        return $id;
    }
}

?Route $onClass = Core\Attributes::get<Route>(Users::constructor(...));
if ($onClass != null) { echo "class: ", $onClass->path, "\n"; }

?Route $onMethod = Core\Attributes::get<Route>(Users::show(...));
if ($onMethod != null) { echo "method: ", $onMethod->path, "\n"; }

?{name: string} $onProperty = Core\Attributes::get<{name: string}>(Users::constructor(...), "name");
if ($onProperty != null) { echo "property: ", $onProperty->name, "\n"; }

?{inject: string} $onParameter = Core\Attributes::get<{inject: string}>(Users::show(...), "id");
if ($onParameter != null) { echo "parameter: ", $onParameter->inject, "\n"; }

array<Audited> $markers = Core\Attributes::all<Audited>(Users::constructor(...));
echo "literals satisfying {}: ", Core\Arr::count($markers), "\n";
```
```output
class: /users
method: /users/{id}
property: user_name
parameter: repo
literals satisfying {}: 2
```

The two spellings of an attribute:

- **Named**: `#[Name(field: value, …)]`. `Name` resolves like any other name (through `use` and the
  file's namespace) and must be a `type` alias whose right-hand side is a shape; a class, an alias
  of a scalar, or an undeclared name is refused. The literal is checked against that shape the way
  any shape-typed binding is: every field the shape declares must be present at its type, and
  extra fields are allowed. `#[Name]` with no list attaches an empty literal, which satisfies an
  alias declaring no fields (`type Audited = {};`). `Name` may also be `Owner::Name`, an alias
  declared inside an interface, class or enum, written the same way as in a type:
  `#[Page::Meta(title: "Home")]`. A name the compiler acts on (below) is never written this way.
- **Bare**: `#[{field: value, …}]` — a literal with no name and nothing to check it against.

Rules that hold for both:

- **Every value is a compile-time constant**: a literal (`int`, `float`, `string`, `bool`, `null`),
  an array or shape literal of constants, a class constant or an enum case. A variable, a call, a
  `new` or an interpolated string is refused where it is written. A `secret` class constant cannot
  reach a payload.
- **Where an attribute may sit**: in front of a `class`, `interface`, `enum`, a method, a property, a
  class constant, and a parameter. Not in front of a statement or a local declaration — `#[...]`
  there is a parse error.
- **Several on one declaration**: stack them on separate lines, or write a comma-separated list in
  one bracket, `#[A(n: 1), B(s: "x")]`. The same name may repeat.
- `#` alone starts a comment; only `#[` opens an attribute.

```nvs error
<?nvs
type Cache = {key: string};
string $host = "example.test";

class Page {
    #[Cache(key: "host-" . $host)]
    public function show(): int { return 1; }
}
```
```output
an attribute's field value is not a compile-time constant
```

# Reading attributes back: `Core\Attributes::get` and `all`

Retrieval is **structural**: `get<T>` answers the one attached literal that satisfies the shape `T`
written at the call site, whether it was attached under a name or bare, and `all<T>` answers every
match in declaration order. Both are resolved while compiling — the call is replaced by the
payload, so nothing is reflected on at run time.

- The target is a first-class-callable reference written at the call: `Users::show(...)` for a
  method, `Users::constructor(...)` for the class itself (a class with no written constructor still
  has one). A second argument, a literal member name, selects a property or a parameter of that
  target. A computed member name answers `null`; a computed target is refused.
- `get<T>` answers `?T` — `null` when nothing matches — and is a compile error when two literals
  match; that is what `all<T>` is for, and it answers `[]` when nothing matches.
- Matching is width subtyping, so a literal with extra fields satisfies a narrower shape, and the
  empty shape `{}` is satisfied by **every** attached literal — a bare marker `#[Audited]` is
  therefore not distinguishable from any other attribute by retrieval; give a marker a field.
- A payload may hold a class constant, an enum case or a `Foo::class`, and each is retrieved as the
  value a read of that same name inlines — folded in the scope the attribute was *written* in, not
  the one it is read from. A constant whose own declaration folds to nothing is the one refusal
  left.
- PHP's `ReflectionClass::getAttributes()` and `ReflectionAttribute::newInstance()` do not exist; a
  retrieved payload is a plain shape value, read with `->`.

```nvs
<?nvs
type Tag = {label: string};
type Route = {path: string};

#[Tag(label: "fast")]
#[Tag(label: "safe")]
class Engine {}

// `all<Tag>` returns every attribute that matches the shape `Tag`, in the order they are written.
array<Tag> $tags = Core\Attributes::all<Tag>(Engine::constructor(...));
foreach ($tags as Tag $tag) {
    echo $tag->label, "\n";
}

// `get<Route>` returns `null`, because `Engine` has no attribute with a `path` field.
?Route $route = Core\Attributes::get<Route>(Engine::constructor(...));
echo $route?->path ?? "no route", "\n";
```
```output
fast
safe
no route
```

# The names the compiler acts on

Everything above is inert metadata. A closed set of `Core`-owned names is different: when an
attribute's name *resolves* to one of them, the compiler reads it — builds a codec, a route row, a
command row or a test row — and checks its payload against that attribute's own option roster,
refusing an unknown field, a mistyped one, or one given twice. The match is by resolved name, so a
userland `type Route = {…}` declares an ordinary attribute and never a route, and `#[Core\Route]`
and `use Core\Route; … #[Route]` are the same attribute.

```nvs error
<?nvs
class Home {
    // `Core\Route` has no `colour` field, so this does not compile.
    #[Core\Route(path: "/", method: Core\Http\Method::Get, colour: "red")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function show(): string {
        return "home";
    }
}
```
```output
`colour` is not an option of `#[Route]`
```

<!-- generated: attributes -->

`#[Core\Test]`, `#[Core\Test\Fixture]` and `#[Core\Test\TestWith]` belong to [testing](#lang-testing).
The rest are below.

# `#[Core\Json\Derive]` and `#[Core\Json\Field]`: a class with a JSON codec

`#[Core\Json\Derive]` on a class gives it an encoder and a decoder, so `Core\Json::encode` accepts
its instances and `Core\Json::decodeAs<T>` builds them. Without the attribute both refuse the class
with a `LogicError` — there is no `JsonSerializable` and no automatic encoding of an object.

```nvs
<?nvs
use Core\Json;
use Core\Json\Field;

#[Core\Json\Derive]
class Address {
    public string $city;
    public function constructor(string $city) { $this->city = $city; }
}

#[Core\Json\Derive]
class User {
    public string $name;
    public int $age;
    #[Field(name: "is_admin")]
    public bool $isAdmin;
    #[Field(skip: true)]
    public int $cachedHash;
    public Address $address;
    public ?int $score;

    public function constructor(string $name, int $age, bool $isAdmin, Address $address, ?int $score) {
        $this->name = $name;
        $this->age = $age;
        $this->isAdmin = $isAdmin;
        $this->cachedHash = 0;
        $this->address = $address;
        $this->score = $score;
    }
}

echo Json::encode(new User("ada", 36, true, new Address("london"), null)), "\n";
```
```output
{"name":"ada","age":36,"is_admin":true,"address":{"city":"london"},"score":null}
```

- **The fields are the class's declared instance properties, in declaration order**, whatever
  their visibility — written in the class body or promoted in the `constructor`, each in the place
  it is written. A `secret` or `lateinit` property is refused on a deriving class.
- **Every field is a constructor parameter of the same name and type** (a decode is an ordinary
  `new`), unless it is `#[Field(skip: true)]`. A field with no matching parameter, or one whose
  parameter has another type, is refused.
- `#[Core\Json\Field]` has exactly two options: `name:` renames the wire key, `skip:` leaves the
  property out of the document entirely. There is no class-wide naming policy.
- A `?T` field always appears in the document, as `null` when it is null; nothing is omitted for
  being null.
- Encoding reaches into nested deriving classes, arrays and enums (an enum encodes as its backing
  value). **Decoding, in this build, handles fields of `string`, `int`, `uint`, `float`, `bool`,
  another deriving class and `?T` of those**; `decodeAs<T>` over a class with an array or enum
  field is a fatal error at run time. A nested field's issues carry the path that reaches them —
  `address.city`, or `1.address.city` inside a list.

A decode checks the whole document before it constructs anything, and one failure carries every
bad field:

```nvs
<?nvs
#[Core\Json\Derive]
class Row {
    public string $name;
    public ?int $rank;
    #[Core\Json\Field(name: "is_live")]
    public bool $live;

    public function constructor(string $name, ?int $rank, bool $live) {
        $this->name = $name;
        $this->rank = $rank;
        $this->live = $live;
    }
}

var $row = Core\Json::decodeAs<Row>("{\"name\":\"ada\",\"rank\":null,\"is_live\":true,\"extra\":1}");
echo $row->name, " ", $row->rank ?? "unranked", "\n";

try {
    Core\Json::decodeAs<Row>("{\"rank\":\"first\",\"is_live\":null}");
} catch (ParseError $bad) {
    foreach ($bad->issues as {path: string, message: string} $issue) {
        echo $issue->path, ": ", $issue->message, "\n";
    }
}
```
```output
ada unranked
name: required field missing
rank: expected int, found a string
is_live: null is not permitted
```

- A key the class does not declare is ignored. A missing key, a value of the wrong type, and
  `null` where the type is not `?T` are each an issue; the issue's `path` is the *wire* key.
- A malformed document is one issue with an empty path. Either way the throw is a `ParseError`
  whose `issues` is an array of `{path: string, message: string}`.

```nvs error
<?nvs
#[Core\Json\Derive]
class User {
    public string $name;
    public int $age;
    public function constructor(string $age) { $this->name = ""; $this->age = 0; }
}
```
```output
`$name` is a `#[Json\Derive]` field with no constructor parameter
```

# `#[Core\Route]` and `#[Core\Access]`: the route table

`#[Core\Route(path:, method:, name:)]` on a method declares a route, and every route needs a
sibling `#[Core\Access(allow: …)]` on the same method. The compiler collects every route in the
program — through the same `autoload` scan that finds classes nothing names — into one table, checks
it, and answers `Core\Router::url` from it.

```nvs file=routes/Users.nvs
<?nvs
namespace App;

class Users {
    #[Core\Route(path: "/users/{id}", method: Core\Http\Method::Get, name: "Users::show")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function show(uint $id): string {
        return "user " . $id;
    }

    #[Core\Route(path: "/users", method: Core\Http\Method::Post, name: "Users::create")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function create(): string {
        return "created";
    }
}
```
```toml file=nvs.toml
[[app]]
root = "."
origin = "https://example.test"
```
```nvs
<?nvs
autoload 'App' from './routes';

echo Core\Router::url("Users::show", ["id" => 7]), "\n";
echo Core\Router::url("Users::create", []), "\n";
echo Core\Router::urlAbsolute("Users::show", ["id" => 7]), "\n";
```
```output
/users/7
/users
https://example.test/users/7
```

**The server matches a request. It never calls a route method.** `nvs serve` matches every
request against the route table once, before the program runs. `Core\Request::route()` returns
that match, or `null` when no route has this verb and path. The server then runs the mount's entry
file for every request, matched or not. This is how the router is designed, and it is not a
missing feature:

- The server does not call the method that carries the `#[Core\Route]`, and it does not use that
  method's return value. The entry file reads the match and calls the method.
- The server sends no `404` and no `405` of its own. The entry file sets them with
  `Core\Response::setStatus`. `Core\Router::methodsFor` returns the verbs a path has: an empty
  array is the `404`, and any other array is the `Allow` header of a `405`.
- The CSRF check on `Post`, `Put`, `Patch` and `Delete` is the one decision the server enforces
  on a matched route. The `allow:` value of `#[Core\Access]` is recorded and not enforced. The
  entry file enforces it: `access()` on the match returns the full name of the `allow:` constant,
  such as `Core\Audience::Public`. Check it once, before the `switch`, and treat `null` as denied.
- A `Core\Router\Match` has a name, the converted captures, the verb and the access decision. It
  has nothing that can be called, so a program dispatches with one `switch` on `name()`. Give
  every route it dispatches a `name:`.

`Core\Router::match` asks the same table about a verb and a path the program chooses, so this
example runs from the command line. Under `nvs serve` the entry file reads `Core\Request::route()`
in its place.

```nvs
<?nvs
class Users {
    #[Core\Route(path: "/users/{id}", method: Core\Http\Method::Get, name: "Users::show")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function show(uint $id): string { return "user " . $id; }

    #[Core\Route(path: "/users", method: Core\Http\Method::Post, name: "Users::create")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function create(): string { return "created"; }
}

class App {
    public static function answer(Core\Http\Method $method, string $path): string {
        var $match = Core\Router::match($method, $path);
        if ($match == null) {
            return Core\Router::methodsFor($path) == [] ? "404" : "405";
        }
        if ($match->access() != "Core\\Audience::Public") {
            return "403";
        }
        var $users = new Users();
        switch ($match->name()) {
            case "Users::show":
                return $users->show($match->param("id") as uint);
            case "Users::create":
                return $users->create();
        }
        return "404";
    }
}

echo App::answer(Core\Http\Method::Get, "/users/7"), "\n";
echo App::answer(Core\Http\Method::Post, "/users"), "\n";
echo App::answer(Core\Http\Method::Delete, "/users"), "\n";
echo App::answer(Core\Http\Method::Get, "/nothing"), "\n";
```
```output
user 7
created
405
404
```

A route method is an ordinary method that may also be called directly.

The `#[Route]` payload:

- `path:` — a string starting with `/`. Segments are literals or captures: `{name}` matches one
  segment, `{name?}` matches one or none and is allowed only as the last segment, `{name...}`
  matches everything that remains, its own `/`s included. A literal segment beats a capture, so
  `/users/new` and `/users/{id}` coexist in any order.
- `method:` — a case of `Core\Http\Method`.
- `name:` — optional and never derived; it is what `Core\Router::url` looks a route up by, and must
  be unique across the program.
- The same `path` under the same `method` twice is a duplicate route, wherever the two are
  declared. Two `#[Route]`s on one method are two routes served by one implementation — the way
  to serve `Get` and `Post` from one method.

**Captures bind to parameters** by name: every `{name}` needs a parameter `$name`, and that
parameter's declared type is what the segment converts to — `string`, `int`, `uint`, `decimal`,
`bool`, an enum, a union of string or int literals (`"en"|"de"`), or a class implementing `Parses`,
which `Core\Uuid` is one of and a class of your own is another. A `float`
parameter is refused. A `{name?}` parameter needs a default. A `{name...}` parameter is a
`string`. The converted value is not `tainted`.

**`#[Core\Query]`** on a parameter of a route method binds it from the query string under the
parameter's own name instead of from the path, converting to its declared type the same way; away
from a route method it is refused.

```nvs
<?nvs
enum Lang { En, De }

class Docs {
    #[Core\Route(path: "/{lang}/docs/{page?}", method: Core\Http\Method::Get, name: "docs")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function page(Lang $lang, uint $page = 1, #[Core\Query] string $q = ""): string {
        return "docs";
    }

    #[Core\Route(path: "/files/{path...}", method: Core\Http\Method::Get, name: "file")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function file(string $path): string {
        return "file";
    }
}

echo Core\Router::url("docs", ["lang" => Lang::De]), "\n";
echo Core\Router::url("docs", ["lang" => Lang::En, "page" => 3, "q" => "a b"]), "\n";
echo Core\Router::url("file", ["path" => "a/b c.txt"]), "\n";
```
```output
/De/docs
/En/docs/3?q=a+b
/files/a/b%20c.txt
```

`Core\Router::url` and `urlAbsolute`:

- The route name is a **string literal at the call**, checked against the table: an unknown name
  is a compile error, and so is a `$params` that leaves a required capture unfilled or names a key
  that is neither a capture nor a `#[Query]` parameter. A name held in a variable is not checked
  and, in this build, throws a `RuntimeError` for every name.
- Each capture value is percent-encoded into its own segment; `{name...}` keeps its `/`s; an
  omitted `{name?}` drops its segment; `#[Query]` keys become the query string; a `null` value is
  dropped. An enum value substitutes the text its cases are spelled by: the written backing value
  where every case the capture admits wrote one, and the case name where any of them counted on
  from the case before — the same spelling the route matches on, so a link is never a path the
  router would not claim. A value outside the capture's set is a compile error.
- `urlAbsolute` puts the configured origin in front — `[[app]] origin` in `nvs.toml` — and throws
  a `RuntimeError` when none is configured. The origin is never read from a request header.

**`#[Core\Access(allow: …, csrf?: bool)]`** is required beside every `#[Route]`: a route with no
access decision does not compile, so an open route says so with `Core\Audience::Public` rather
than by omission. `allow:` is any constant — `Core\Audience::Public` or the application's own
enum case or class constant; the compiler records the decision and does not interpret it. One
`#[Access]` per method, covering every `#[Route]` the method carries; two are refused, and one
without `allow:` is refused. The requirement runs both ways: an `#[Access]` on a method that
carries no `#[Route]` is refused too (`E0788`), because the route table is the only thing that ever
reads a decision, so one written away from a route guards nothing and never will.
`csrf: false` opts the method out of the CSRF check that covers
`Post`, `Put`, `Patch` and `Delete`, and is refused on a method whose routes are all safe verbs.

```nvs error
<?nvs
class Users {
    #[Core\Route(path: "/users", method: Core\Http\Method::Get, name: "Users::index")]
    public function index(): string { return "users"; }
}
```
```output
the route `Users::index` declares no access decision
```

# `#[Core\Api]` and the OpenAPI document

`nvs build --openapi main.nvs` writes an OpenAPI 3.1 document for the program's route table to
standard output (the CLI chapter has the command). Each route is an operation: its path captures
and `#[Query]` parameters become parameters with schemas from their declared types, the return type
becomes the `200` response, and the method's `///` doc comment supplies the summary (first
sentence) and description (the rest).

```nvs
<?nvs
class Users {
    /// Show one user. Looks the user up by id.
    #[Core\Route(path: "/users/{id}", method: Core\Http\Method::Get, name: "Users::show")]
    #[Core\Access(allow: Core\Audience::Public)]
    #[Core\Api(tags: ["users"])]
    public function show(uint $id): string { return "user " . $id; }
}
echo "routes declared", "\n";
```
```output
routes declared
```

For that file `nvs build --openapi main.nvs` prints:

```json
{
  "info": {"title": "main", "version": "0.0.0"},
  "openapi": "3.1.0",
  "paths": {
    "/users/{id}": {
      "get": {
        "description": "Looks the user up by id.",
        "operationId": "Users::show",
        "parameters": [
          {"in": "path", "name": "id", "required": true, "schema": {"minimum": 0, "type": "integer"}}
        ],
        "responses": {
          "200": {"content": {"application/json": {"schema": {"type": "string"}}}, "description": "success"}
        },
        "summary": "Show one user.",
        "tags": ["users"]
      }
    }
  }
}
```

`#[Core\Api(...)]` sits beside a `#[Route]` (away from one it is refused) and may add what the
declaration cannot say: `tags:` and `security:` as arrays of strings, `errors:` as an array of
`{status: 404, type: NotFound::class}` entries naming classes the program declares, and `example:`
as a shape literal whose keys are properties of the return type. Each is checked against the
declaration while compiling and then written into the operation: `tags` and `security` as their own
members, an `errors` entry as a response of its own described by its class, and `example` beside the
`200` response's schema. An operation whose method declares no `#[Api]` carries none of them.

# `#[Core\Command]` and `#[Core\Option]`: the command table

`#[Core\Command(name:, about:)]` on a static method declares a command-line command, and
`#[Core\Option(short:, long:, about:)]` on a parameter makes that parameter an option. The
compiler collects every command into one table and checks it. **Nothing dispatches to it in this
build**: there is no `Core\Command::run`, and no `nvs` subcommand invokes a program's commands. What
exists is the checked table and the ordinary ways to read it — `Core\Program::implementing<I>()`
over an interface the command classes implement, and `Core\Attributes::get` on the methods.

```nvs file=commands/Command.nvs
<?nvs
namespace App;

interface Command {
    public function usage(): string;
}
```
```nvs file=commands/Greet.nvs
<?nvs
namespace App;

class Greet implements Command {
    #[Core\Command(name: "greet", about: "Say hello")]
    public static function greet(
        tainted string $name,
        #[Core\Option(short: "l", about: "Shout it")] bool $loud,
    ): void {
        echo $name, $loud ? "!" : "", "\n";
    }

    public function usage(): string { return "greet <name> [--loud]"; }
}
```
```nvs file=commands/Migrate.nvs
<?nvs
namespace App;

class Migrate implements Command {
    #[Core\Command(name: "migrate", about: "Apply pending migrations")]
    public static function migrate(
        #[Core\Option(about: "Print what would happen")] bool $pretend,
    ): uint {
        if ($pretend) { return 0; }
        return 1;
    }

    public function usage(): string { return "migrate [--pretend]"; }
}
```
```nvs
<?nvs
autoload 'App' from './commands';

foreach (Core\Program::implementing<App\Command>() as App\Command $command) {
    echo $command->usage(), "\n";
}

?{about: string} $about = Core\Attributes::get<{about: string}>(App\Greet::greet(...));
if ($about != null) { echo "greet: ", $about->about, "\n"; }
?{about: string} $loud = Core\Attributes::get<{about: string}>(App\Greet::greet(...), "loud");
if ($loud != null) { echo "--loud: ", $loud->about, "\n"; }
```
```output
greet <name> [--loud]
migrate [--pretend]
greet: Say hello
--loud: Shout it
```

- `name:` is required — it is the word a command line selects the command by — and unique across
  the program; `about:` is the one-line description. Two `#[Command]`s on one method are two names
  for one command.
- A parameter is a **positional argument unless it carries `#[Option]`**; nothing is inferred from
  a default or a type. Positional text is raw input, so the examples declare it `tainted string`.
  A `bool` option is a flag. `short:` is the one-letter spelling, `long:` the spelling used when
  the parameter's own name is not it; two options of one command with the same spelling are
  refused. `#[Option]` away from a `#[Command]` method is refused.
- An option or positional parameter must have a type an argument's text converts to — the same
  list a route capture accepts (`string`, `int`, `uint`, `decimal`, `bool`, an enum, a union of
  literals, a class implementing `Parses`); anything else is refused.
- The method is **`static`** and returns `void` (exit status 0) or `uint` (the exit status). Both
  are refused where they are not met (`E0789`): a command is dispatched by name off the compiled
  table, which holds no instance to call a handler on, and what a handler answers with is the
  status `Core\Command::run` returns.

```nvs error
<?nvs
class Deploy {
    #[Core\Command(name: "deploy", about: "Push the build")]
    public static function deploy(#[Core\Option] float $ratio): void {}
}
```
```output
`float` is not a type an option of `deploy` can be given at
```

# `Core\Program::implementing<I>()`: every class implementing an interface

`Core\Program::implementing<I>()` expands, while compiling, to an array literal of `new C()` for
every non-abstract class in the program that implements the interface `I`, **sorted by
fully-qualified name**, so the order never depends on the file system. The classes are found by the
`autoload` scan ([programs](#lang-programs)) even when nothing else names them, which is how the
route and command tables find theirs too.

- `I` must be an interface; a class or a shape is refused. Every implementing class needs a
  constructor callable with no arguments — dependencies arrive through `I`'s own methods.
- The instances are built where the call stands, once per evaluation, like any other `new`.
- An interface nothing implements answers `[]`.

```nvs
<?nvs
interface Greeter {
    public function greet(): string;
}

class English implements Greeter {
    public function greet(): string {
        return "hello";
    }
}

class German implements Greeter {
    public function greet(): string {
        return "hallo";
    }
}

// The array has one new object of each class, sorted by class name.
foreach (Core\Program::implementing<Greeter>() as Greeter $greeter) {
    echo $greeter->greet(), "\n";
}
```
```output
hello
hallo
```

# `Core\Program::implementingWith<I, T>($member)`: every implementor with one attribute

`Core\Program::implementingWith<I, T>($member)` expands, while compiling, to the same array with one
attribute joined to each class. Every row is `{instance: I, attribute: ?T}`: the instance, and the one
attribute on that class's own `$member` whose fields satisfy the shape `T` — the same structural match
`Core\Attributes::get<T>` makes — or `null` when the class carries none. This is how a framework reads
the attributes of the classes it discovered: inside a loop over `implementing<I>()` the variable is
typed as `I`, so there is no class name to write into a retrieval, and the compiler does the join
instead.

- `$member` is a method name, a property or constructor-parameter name, or the empty string for the
  attributes on the class itself. A name some implementor does not declare is an error naming that
  class.
- Two matching attributes on one class are an error naming that class; narrow the shape, or read that
  class with `Core\Attributes::all<T>`.
- `T` is a shape, inline or a `type` alias, exactly as for `Core\Attributes::get<T>`.

```nvs
<?nvs
interface Page {
    public function render(): string;
}

class Index implements Page {
    #[Core\Route(path: "/", method: Core\Http\Method::Get, name: "index")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function render(): string {
        return "home";
    }
}

class Contact implements Page {
    #[Core\Route(path: "/contact", method: Core\Http\Method::Get, name: "contact")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function render(): string {
        return "contact us";
    }
}

class Draft implements Page {
    public function render(): string {
        return "not routed";
    }
}

// Each page beside the path its route declares, so the path is written once.
foreach (Core\Program::implementingWith<Page, {path: string}>("render") as {instance: Page, attribute: ?{path: string}} $row) {
    echo $row->attribute?->path ?? "(no route)", " -> ", $row->instance->render(), "\n";
}
```
```output
/contact -> contact us
(no route) -> not routed
/ -> home
```
