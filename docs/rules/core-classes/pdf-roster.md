`Novis\Pdf` is five classes a program names — `Document` renders, `Page` is one page's facts, `Rendered`
is a render's result, `Pages` merges and `Reader` reads — and `Engine`, the manifest class whose static
methods are the WIT exports (`wit/pdf.wit`), the way `Novis\Image\Codec` is the image component's.

| Member | What it does |
|---|---|
| `Document::fromHtml(Core\Html\Markup\|string $html): Document` | the document to render |
| `->asset(string $name, bytes $data): Document` | adds one named asset (`rule:core-classes/pdf-render-has-no-io`) |
| `->header(callable $fn, ?{height?: string} $options = null): Document` | a function from `Page` to `Core\Html\Markup`, drawn in the top margin of each page (`rule:core-classes/pdf-header-is-a-page-function`) |
| `->footer(callable $fn, ?{height?: string} $options = null): Document` | the same, in the bottom margin |
| `->slots(callable $fn): Document` | a function from `Page` to `array<string, Core\Html\Markup>`, filling each `<slot name>` in a repeated table row (`rule:core-classes/pdf-tables-across-pages`) |
| `->standard(Document::Standard $standard): Document` | `PdfA2b`, `PdfA3b` or `PdfUa1` (`rule:core-classes/pdf-standards-and-passwords`) |
| `->invoiceXml(string $xml): Document` | the Factur-X XML (`rule:core-classes/pdf-factur-x-is-the-one-attachment`) |
| `->password(string $password): Document` | encrypts the output with AES-256 |
| `->created(Core\Time\Instant $at): Document` | the creation time in the metadata; absent, the file has none |
| `->render(): Rendered` | one render: one call, two with a header, footer or slot function |
| `Page->number`, `->count`, `->isFirst()`, `->isLast()`, `->name` | the page's number from 1, the document's page count, and its named page or `null` |
| `Page->string(string $name): ?string` | the page's `string-set` value |
| `Page->marks(string $name): array<string>`, `->marksUpTo(string $name): array<string>` | the `data-mark` values on this page, and on every page up to it |
| `Rendered->bytes`, `->pageCount` | the file and its page count |
| `Rendered->dropped`, `->missingGlyphs`, `->overflows` | what the render could not do (`rule:core-classes/pdf-css-subset`) |
| `Pages::of(bytes $pdf, ?array<uint> $pages = null): Pages`, `Pages::merge(array<Pages> $parts): bytes` | merging, splitting and reordering (`rule:core-classes/pdf-reader-and-merge`) |
| `Reader::open(bytes $pdf): Reader`, `->pageCount`, `->metadata`, `->attachments()`, `->invoiceXml()`, `->text(uint $page): string` | reading a PDF |

`Document` is a value: each method returns a new `Document`, so a base document with its assets and
footer is shared and varied without one render changing another. `Standard` is declared in
`Document`, the one class that uses it (`rule:enums/an-enum-lives-in-the-class-that-uses-it`).
`Page` and `Rendered` are made only by the component.

**The errors**, each naming what it refuses:

| Throws | When |
|---|---|
| `LogicError` | a reference the asset map does not resolve; a broken standard; an invoice XML outside PDF/A-3b; a password beside a PDF/A standard; a page number outside its document in `Pages::of` or `Reader::text` |
| `ParseError` | a source PDF, font or image that does not parse; an encrypted source PDF; an invoice XML that is not well-formed |
| `RuntimeError` | page numbers that do not settle after five layout passes (`target-counter()`, `counter(pages)`) |

The request's memory cap and deadline end a render that exceeds them as they end any call.

- **Memory:** a `Document` holds its HTML and its assets until it is freed; the render's own memory is
  `rule:core-classes/pdf-render-has-no-io`'s.

**Partly on disk.** `extensions/pdf/manifest.json` declares `Engine` and `Standard`, and
`extensions/pdf/nvs/` holds `Document` without `header`, `footer` and `slots`, and `Rendered`. `Page`,
`Pages` and `Reader` are not written. The `pdf` goal builds `Page`, `header` and `footer` in its
Stage 10, `slots` in Stage 11, and `Pages` and `Reader` in Stage 13.

**Declined:**
- **A `Pdf` facade with static `render($html, $options)`.** The options a render takes — assets, three
  functions, a standard, a password — are an options bag too large to read, and a base document could
  not be shared.
- **`data-mark-<name>` attributes.** The editor completes fixed attribute names only, so one
  `data-mark="name:value"` attribute is what completes and explains itself.
