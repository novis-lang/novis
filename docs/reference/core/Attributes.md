---
summary: reads the attributes attached to a class, a method, a property or a parameter — each one an anonymous object, read by the shape it satisfies, at compile time
keywords: ReflectionAttribute, getAttributes, newInstance, attribute, metadata, annotation, #[...], anonymous object
---

`Core\Attributes::get<T>` returns the one attribute on a target whose payload object satisfies the
shape `T` written at the call site, or `null`; `all<T>` returns every one in declaration order, as
`array<T>`. Retrieval is structural. A payload object attached bare (`#[{audit: true}]`) or under a
`type` name (`#[Route(path: "/x")]`) is found by any shape it satisfies, extra fields included. The target
is a class's `Class::constructor(...)` or a method's `Class::method(...)` reference, and the second
argument names one of its properties or parameters; the call is resolved in `nvs check`, so a
computed member name answers nothing, and two matches for `get` is a compile error naming `all`.

```nvs
<?nvs
type Route = {path: string};

#[Route(path: "/users")]
#[{audit: true}]
class Controller {
    #[{column: "name", unique: true}]
    public string $name = "";

    #[Route(path: "/show")]
    public static function show(#[{inject: "repo"}] int $id): int { return $id; }
}

?Route $route = Core\Attributes::get<Route>(Controller::constructor(...));
echo "class: ", $route?->path ?? "none", "\n";
?{audit: bool} $audit = Core\Attributes::get<{audit: bool}>(Controller::constructor(...));
echo "audit: ", ($audit != null ? "yes" : "no"), "\n";
?{column: string} $column = Core\Attributes::get<{column: string}>(Controller::constructor(...), "name");
echo "column: ", $column?->column ?? "none", "\n";
?Route $method = Core\Attributes::get<Route>(Controller::show(...));
echo "method: ", $method?->path ?? "none", "\n";
?{inject: string} $param = Core\Attributes::get<{inject: string}>(Controller::show(...), "id");
echo "param: ", $param?->inject ?? "none", "\n";
array<Route> $none = Core\Attributes::all<Route>(Controller::show(...), "id");
echo "none: ", Core\Arr::count($none), "\n";
```
```output
class: /users
audit: yes
column: name
method: /show
param: repo
none: 0
```
