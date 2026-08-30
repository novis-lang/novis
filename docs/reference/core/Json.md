---
summary: JSON in and out — `encode` any value, `decode` to `mixed`, `decodeAs<T>` straight into a class that declares its codec
keywords: json_encode, json_decode, json_validate, json_last_error, json_last_error_msg, JSON_PRETTY_PRINT, JSON_UNESCAPED_UNICODE, JSON_THROW_ON_ERROR, JsonSerializable, hydration, typed decoding, Derive, Field
---

`Core\Json::encode` writes any scalar, array or instance of a class carrying `#[Core\Json\Derive]`
as JSON: a list becomes a JSON array, any other array a JSON object with its insertion order kept,
and an instance of a class without the attribute is refused. `decode` answers `mixed` — an object
as a string-keyed array — and throws `ParseError` on a malformed or too-deep document, so there is
no `json_last_error`. `decodeAs<T>` reads a document straight into a class that carries
`#[Core\Json\Derive]`, checking every declared field against its type — `?T` is the only way a
field admits `null` — and reporting every failure at once in one `ParseError`'s `issues`.
`#[Core\Json\Field(name: "…")]` gives one field its wire name.

```nvs
<?nvs
#[Core\Json\Derive]
class User {
    public string $name;
    public ?int $age;
    #[Core\Json\Field(name: "is_admin")]
    public bool $admin;
    public function constructor(string $name, ?int $age, bool $admin) {
        $this->name = $name;
        $this->age = $age;
        $this->admin = $admin;
    }
}
echo Core\Json::encode(["id" => 7, "tags" => ["a", "b"], "ok" => true]), "\n";
var $doc = Core\Json::decode("{\"n\": 3.5, \"list\": [1, 2]}");
echo Core\Json::encode($doc, {pretty: true}), "\n";
User $u = Core\Json::decodeAs<User>("{\"name\":\"Ada\",\"age\":null,\"is_admin\":true}");
echo $u->name, " ", $u->age ?? "unknown", " ", Core\Json::encode($u), "\n";
echo Core\Json::isValid("{oops}") ? "valid" : "invalid", "\n";
try {
    Core\Json::decodeAs<User>("{\"name\":1}");
} catch (ParseError $e) {
    echo Core\Arr::count($e->issues), " issues\n";
}
```
```output
{"id":7,"tags":["a","b"],"ok":true}
{
  "n": 3.5,
  "list": [
    1,
    2
  ]
}
Ada unknown {"name":"Ada","age":null,"is_admin":true}
invalid
3 issues
```
