---
summary: the elements of a JSON array, which `Core\Json::decodeEach` returns. You read them one at a time with a `foreach` loop
keywords: large JSON file, stream JSON, read JSON array, JSON element by element, JSON Lines, NDJSON, import JSON, low memory
---

`Core\Json\Elements<T>` contains the elements of a JSON array. `Core\Json::decodeEach<T>` returns it.
It reads an open `Core\IO\File`, an uploaded file, `Core\Request::bodyStream()`, `bytes`, or a list or
loop that gives `bytes` pieces. A file is read from its first byte. The class has no methods. You read
it with a `foreach` loop.

Each item is one element, as a `T`. `T` can be a class with the `#[Core\Json\Derive]` attribute, a shape
such as `{ name: string, age: int }`, or `mixed`. Each element is checked against `T`. When `T` is
`mixed`, every string in an element is a `tainted string`.

The JSON is read while the loop runs, and only one element is in memory at a time. You can read the
elements only once. A second loop continues after the last element the first loop read. After the last
element, a loop gives no items.

The loop throws a `ParseError` when the JSON is not valid or an element does not match `T`. The
message gives the byte offset. Your loop has already received the elements before it. The loop throws
a `RuntimeError` when the file is closed or cannot be read. `decodeEach` does not close the file. Call
`close()` on the file when the loop is done.

```json file=users.json
[
  {"name": "Ada", "age": 36},
  {"name": "Ben", "age": 41}
]
```
```json file=broken.json
[1, 2, oops]
```
```nvs
<?nvs
Core\IO\File $users = Core\IO::open('users.json', Core\IO::FileMode::Read);
foreach (Core\Json::decodeEach<{ name: string, age: int }>($users) as var $user) {
    echo $user->name, ' is ', $user->age, "\n";
}
$users->close();

// The third element is not valid JSON. The loop prints 1 and 2 first.
Core\IO\File $broken = Core\IO::open('broken.json', Core\IO::FileMode::Read);
try {
    foreach (Core\Json::decodeEach<mixed>($broken) as mixed $n) {
        echo $n as ?int ?? 0, "\n";
    }
} catch (ParseError $e) {
    echo "not valid JSON\n";
}
$broken->close();
```
```output
Ada is 36
Ben is 41
1
2
not valid JSON
```
