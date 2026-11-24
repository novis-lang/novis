---
summary: the request a program is answering — query, form fields, cookies, headers, uploads and the body in one class, every answer of it `tainted`
keywords: request, query string, query parameters, form fields, POST data, cookies, headers, request body, JSON body, multipart, upload, HEAD, request method, path, tainted input
---

`Core\Request` is the whole of what arrived: `method`, `isHead`, `path`, `queryParameter`, `header`, `headers`,
`cookie` and the body readers, plus `route` and `mount` for where the router put it. The host never
fills a variable for the program. No lookup merges the query, the form fields and the cookies into
one. Every member throws `LogicError` in a program that is
not answering a request, so a CLI run, a scheduled script and a job worker say "no request arrived"
rather than answering empty.

**The body is read once, and every reader is one of two kinds.** `body()`, `formField()`, `formFieldsAs<T>()`,
`json()` and `jsonAs<T>()` **buffer**: the first of them fills the request's hold, bounded by
`[limits] request_body`, and any of the five may follow any other in any order and answer the same
octets. `bodyStream()` and
`files()` **stream**: they hand the octets over as they arrive, keep none, and so consume the body — a
reader after one of those is refused, naming the member that consumed it. That refusal is a defect in
the program, never something a peer can provoke.

`json()` decodes the body as one JSON document and answers `mixed`; `jsonAs<T>()` hydrates it straight
into a class carrying `#[Core\Json\Derive]`, exactly as `Core\Json::decodeAs` does, so a handler names
the shape it expects instead of walking an array. Neither consults the `Content-Type` the peer
declared — a header is what a peer wrote, not what a body is — and both throw `ParseError` on a
document that is malformed, too deep, or absent altogether. `json()` keeps what it decoded and
`jsonAs<T>()` hydrates per call, so writing to what a second call answered leaves the first alone.

`queryParametersAs<T>()` and `formFieldsAs<T>()` do the same for the query string and the form. Each
reads what `queryParameter()` and `formField()` read as the type `T` that you write at the call. `T` is an
inline shape, such as `{ page: uint }`, or a class with `#[Core\Json\Derive]`. Each field that `T` names
is converted with `as`, and the other keys are left out. Without arguments, the method reads every
parameter. This is the only way to read all of them at once. With `{ name: "filter" }`, it reads the
nested values under that one name. A field that is missing, or that `as` cannot convert, gives a
`ParseError`. The error lists every wrong field at its own path, so a form can show all of them.

Everything a peer chose is `tainted`, including the path, every header and the body, so it reaches a
sink only through a launderer named for that sink — `echo` escapes for HTML on its own. A class hydrated
from a body declares that on the fields receiving it: an unqualified `string` property is a compile
error at the `jsonAs<T>()` call rather than a silent laundering.

```nvs skip
<?nvs
#[Core\Json\Derive]
class Author
{
    public tainted string $name;
    public int $tags;

    public function constructor(tainted string $name, int $tags)
    {
        $this->name = $name;
        $this->tags = $tags;
    }
}

if (Core\Request::method() == Core\Http\Method::Post) {
    Author $author = Core\Request::jsonAs<Author>();
    echo 'hello ', $author->name, "\n";
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
