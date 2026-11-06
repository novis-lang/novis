A `Core` member that reads bytes takes them from where the program holds them: bytes in memory, an open
`Core\IO\File`, an upload part or the request body, and, where it reads forward once, a walk of chunks.

**Two aliases on `Core\Bytes` name the sources.**

```nvs
type Source = tainted bytes|Core\IO\File|Core\Request\Part|Core\Request\BodyStream;
type Chunks = array<tainted bytes>|Iterable<tainted bytes>|Iterator<tainted bytes>;
```

A `Source` can give its start without being consumed, and can be held for a reading in any order. `Chunks`
is read once, forward. A plain `bytes` value or chunk widens to its `tainted` form, so both take what a
program built itself. A forward member writes `Core\Bytes::Source|Core\Bytes::Chunks`, so the one thing a
reader needs to know, whether a stream is accepted, is visible in the signature. `string` stays an arm
where a member takes one.

| The member reads | It takes | Members |
|---|---|---|
| the start | `Core\Bytes::Source` | `Core\Mime::detect`, `Core\Request\Part::peek` and `Core\Request\BodyStream::peek` (their own source) |
| once, forward | `Core\Bytes::Source\|Core\Bytes::Chunks` | `Core\Json::decodeEach`, `Core\Csv::rows`, `Core\Hash::of`, `Core\Hash::hmac`, `Core\Compress::compress`, `Core\Compress::decompress`, `Core\Xml::reader`, `Core\Storage::put` |
| in any order | `Core\Bytes::Source` | `Core\Zip`, `Core\Pdf`, `Core\Image` (`open`, `info`, `hash`, `placeholder`, `palette`, `compare`), the spreadsheet `Reader::open` and `Template::open` |
| the whole value, checked before anything is given | `bytes` | `Core\Crypto::open`, `Core\Crypto::sign`, `Core\Crypto::verify`, `Core\Serialize::decode`, `Core\Uuid::fromBytes` |

**The criterion: a member takes a source when it can hold less than its input.** A tree parser holds the
whole document anyway, so `Core\Xml::parse`, `Core\Html::parse`, `Core\Json::decode` and `Core\Csv::parse`
keep `string` or `bytes`. So does `Core\Encoding`, whose output is as large as its input. A future source
joins `Source` only when it can give all three kinds of read. A forward-only source yields `tainted bytes`
chunks and fits `Chunks` as it is.

**Every kind reads a file whole, from its start.** A read of the start and a read in any order put the
file's position back. A forward read leaves it at the end. So `$f->readBytes(4); Core\Hash::of($f, …)`
hashes the whole file. A file that cannot seek, such as a pipe, is read forward from where it is. Its
start is buffered for a read of the start, and a read in any order holds it whole under `[limits] memory`.
A file opened write-only throws a `LogicError` naming its mode.

**A built-in component that parses one whole buffer reads the source whole at its call.** `Core\Pdf` and
`Core\Image` write `Core\Bytes::Source` in their manifest where the WIT has `list<u8>`, and the host reads
the source through the byte source's range read, under `[limits] memory`, and passes the bytes. A third
party's export cannot declare it, because the host lends a third party's `bytes` by ranges instead.
`Pages::of` and `Image::open` keep the source and the host reads it at `merge`, `encode`, `variants` or
`raw`, so a file is read at that call from its start, and a part must still be the walk's current part.
Reading it at `open` would need a `Core` member that turns a source into whole `bytes`, which is the slow
path `rule:core-api/the-first-call-is-the-best-one` keeps out. Text read from the file, such as a PDF's
text and metadata or an image's EXIF fields, is `tainted`. A value the component computes in a fixed
format, such as an image hash, a placeholder or a drawn diff, is not.

**Taint follows the source** (`rule:security/tainted-qualifier`). A part, a body stream and a walk of
`tainted` chunks carry `tainted` into a contagious result, and a `Core\IO\File` carries none, as the text
`Core\IO::read` gives is not tainted. The checker reads it with no run-time cost.

**Memory:** a source holds one reused chunk buffer. A read of a part's start holds what it read until the
walk advances. A read of a part in any order holds the part under `[limits] request_body`, which is what
`readAll` costs, and frees it when the walk advances. A call into `Core\Pdf` or `Core\Image` holds the
source's bytes twice while it runs, once in the host's copy and once in the component. Each member's doc
comment says what it holds per call.

**Declined:**
- **An `Iterable<bytes>` given to a member that reads the start or reads in any order.** Reading the start
  would consume it, and a reading in any order cannot go back. It does not compile.
- **A temporary file for a held part.** An upload has no temporary file
  (`rule:http-server/an-upload-is-received-only-through-files`).
- **A streaming `Core\Crypto`.** `open` must check the tag before it gives a byte, and Ed25519's `sign` and
  `verify` hash the message twice.
- **Widening the tree parsers and `Core\Encoding`**, by the criterion above.
- **`Core\IO\File::chunks()`.** Every forward member takes a `File` directly.
- **An interface a Novis class implements to be a source.** A native reader would then run compiled code
  inside a parse, which `rule:core-classes/json-whole-and-stream` avoids. A Novis generator fits `Chunks`
  already.
- **Removing `Core\Hash::stream` and `Core\Compress::compressor`/`decompressor`.** They stay, for data the
  program makes one piece at a time.
