---
summary: the request a program is answering — `$_GET`, `$_POST`, `$_COOKIE`, `$_FILES` and `php://input` as one class, every answer of it `tainted`
keywords: $_GET, $_POST, $_COOKIE, $_FILES, $_SERVER, $_REQUEST, superglobal, filter_input, getallheaders, php://input, file_get_contents, json_decode, request body, form fields, multipart, upload, HEAD, request method
---

`Core\Request` is the whole of what arrived: `method`, `isHead`, `path`, `query`, `header`, `headers`,
`cookie` and the body readers, plus `route` and `mount` for where the router put it. There are no
superglobals — no variable is ever populated by the host — and `$_REQUEST`, which merged three sources
into one lookup, has no replacement of any kind. Every member throws `LogicError` in a program that is
not answering a request, so a CLI run, a scheduled script and a job worker say "no request arrived"
rather than answering empty.

**The body is read once, and every reader is one of two kinds.** `body()`, `post()`, `json()` and
`jsonAs<T>()` **buffer**: the first of them fills the request's hold, bounded by `[limits] request_body`,
and any of the four may follow any other in any order and answer the same octets. `bodyStream()` and
`files()` **stream**: they hand the octets over as they arrive, keep none, and so consume the body — a
reader after one of those is refused, naming the member that consumed it. That refusal is a defect in
the program, never something a peer can provoke.

`json()` decodes the body as one JSON document and answers `mixed`; `jsonAs<T>()` hydrates it straight
into a class carrying `#[Core\Json\Derive]`, exactly as `Core\Json::decodeAs` does, so a handler names
the shape it expects instead of walking an array. Neither consults the `Content-Type` the peer
declared — a header is what a peer wrote, not what a body is — and both throw `ParseError` on a
document that is malformed, too deep, or absent altogether. `json()` keeps what it decoded and
`jsonAs<T>()` hydrates per call, so writing to what a second call answered leaves the first alone.

Everything a peer chose is `tainted`, including the path, every header and the body, so it reaches a
sink only through a launderer named for that sink — `echo` escapes for HTML on its own. A class hydrated
from a body declares that on the fields receiving it: an unqualified `string` property is a compile
error at the `jsonAs<T>()` call rather than a silent laundering.

```nvs skip
<?nvs
#[Core\Json\Derive]
class Author {
    public tainted string $name;
    public int $tags;

    public function constructor(tainted string $name, int $tags) {
        $this->name = $name;
        $this->tags = $tags;
    }
}

if (Core\Request::method() == Core\Http\Method::Post) {
    Author $author = Core\Request::jsonAs<Author>();
    echo "hello ", $author->name, "\n";
}
```

That one needs a request in front of it; `examples/json-body.nvs` is the same program as a fixture, run
with `nvs run --request <file>`, which answers the request a file describes without standing a listener
up. Without one, this is what every member does:

```nvs
<?nvs
try {
    Core\Request::method();
    echo "not reached\n";
} catch (LogicError $none) {
    echo "no request here\n";
}
```
```output
no request here
```
