---
summary: reverse routing — a link to a route by its declared `name`, as a rooted path or with the configured origin in front
keywords: route, url, link, reverse routing, named route, #[Core\Route], origin, mount, router
---

`Core\Router::url` builds a link from a route's `name` as its `#[Core\Route]` declared it: each
`{capture}` in the path is substituted from `$params`, percent-encoded into its own segment. It is
only meaningful in a program that declares routes — the attributes chapter owns the declaration, and
every `#[Core\Route]` needs a sibling `#[Core\Access]`. The name must be a literal, and an unknown
one is a compile error, as is a `$params` array missing a capture or carrying a key that is neither
a capture nor a handler parameter declared `#[Core\Query]` — a declared one becomes the link's query
string, `/users/7?page=2`. `urlAbsolute` is `url` with the `[[app]] origin` from `nvs.toml` in
front, and throws `RuntimeError` when none is configured.

```toml file=nvs.toml
[[app]]
root = "."
origin = "https://example.test"
```
```nvs
<?nvs
class Users {
    #[Core\Route(path: "/users", method: Core\Http\Method::Get, name: "Users::index")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function index(): string { return "every user"; }

    #[Core\Route(path: "/users/{id}", method: Core\Http\Method::Get, name: "Users::show")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function show(uint $id): string { return "user " . $id; }

    #[Core\Route(path: "/users/{id}/files/{path...}", method: Core\Http\Method::Get, name: "Users::file")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function file(uint $id, string $path): string { return $path; }
}

echo Core\Router::url("Users::index", []), "\n";
echo Core\Router::url("Users::show", ["id" => 7]), "\n";
echo Core\Router::url("Users::file", ["id" => 7, "path" => "a b/c.txt"]), "\n";
echo Core\Router::urlAbsolute("Users::show", ["id" => 7]), "\n";
```
```output
/users
/users/7
/users/7/files/a%20b/c.txt
https://example.test/users/7
```
