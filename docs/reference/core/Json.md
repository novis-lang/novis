---
summary: JSON text from values and values from JSON text. `encode` and `decode` work on a whole document, `decodeAs<T>` reads into a class, and `encodeEach` and `decodeEach` work on a long list
keywords: JSON, encode, decode, parse JSON, validate JSON, pretty print, hydration, typed decoding, JSON to class, Derive, Field, stream JSON, large JSON file, JSON Lines, NDJSON
---

`Core\Json::encode` converts a value to JSON text. A list becomes a JSON array. Any other array
becomes a JSON object, and its keys keep their order. An object can be encoded only if its class has
the `#[Core\Json\Derive]` attribute. Any other object throws an error.

`Core\Json::decode` reads JSON text and returns `mixed`. A JSON object becomes an array with string
keys. Text that is not valid JSON throws a `ParseError`. Text that is nested too deep throws one too.

`Core\Json::decodeAs<T>` reads JSON text into a new instance of a class with `#[Core\Json\Derive]`.
Each property is checked against its type. Only a property with a `?` type, such as `?int`, accepts
`null`. If fields are wrong, the `ParseError` has one entry for each of them in its `issues` list.
`#[Core\Json\Field(name: 'is_admin')]` gives a property a different key in the JSON text. Write
`decodeAs<array<User>>` to read a JSON array with one `User` in each element.

`Core\Json::encodeEach` and `Core\Json::decodeEach` work on a long list one element at a time. The
whole document is never in memory at once. `encodeEach` returns a `Core\Json\Pieces`: the JSON text
in pieces of bytes. `decodeEach<T>` reads a file or a stream of bytes and returns a
`Core\Json\Elements<T>`. You read both with a `foreach` loop. Set `lines` to `true` to write or read
JSON Lines, with one value on each line.

```nvs
<?nvs
#[Core\Json\Derive]
class User
{
    public string $name;
    public ?int $age;
    #[Core\Json\Field(name: 'is_admin')]
    public bool $admin;

    public function constructor(string $name, ?int $age, bool $admin)
    {
        $this->name = $name;
        $this->age = $age;
        $this->admin = $admin;
    }
}

echo Core\Json::encode(['id' => 7, 'tags' => ['a', 'b'], 'ok' => true]), "\n";
var $doc = Core\Json::decode("{\"n\": 3.5, \"list\": [1, 2]}");
echo Core\Json::encode($doc, { pretty: true }), "\n";
User $u = Core\Json::decodeAs<User>("{\"name\":\"Ada\",\"age\":null,\"is_admin\":true}");
echo $u->name, ' ', $u->age ?? 'unknown', ' ', Core\Json::encode($u), "\n";
echo Core\Json::isValid('{oops}') ? 'valid' : 'invalid', "\n";
try {
    Core\Json::decodeAs<User>("{\"name\":1}");
} catch (ParseError $e) {
    echo Core\Arr::count($e->issues), " issues\n";
}

// `encodeEach` writes the list in pieces, and `decodeEach` reads the pieces back one user at a time.
array<User> $team = [new User('Ada', 36, true), new User('Ben', null, false)];
foreach (Core\Json::decodeEach<User>(Core\Json::encodeEach($team)) as User $member) {
    echo $member->name, ' ', $member->admin ? 'admin' : 'user', "\n";
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
Ada admin
Ben user
```
