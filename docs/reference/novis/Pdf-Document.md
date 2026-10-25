---
summary: render HTML and CSS as a PDF file, with page headers, footers and the files the HTML uses
keywords: pdf, generate pdf, html to pdf, wkhtmltopdf, invoice, report, page number, header, footer, pdf/a, factur-x
---

`Novis\Pdf\Document` is an HTML document that `render` turns into a PDF file. Each method returns a
new `Document` and does not change the old one. The render reads only the files you add with `asset`
and `data:` URIs, so a name that is not an asset throws a `LogicError`. `render` returns a
`Rendered` with the file, its page count, and the CSS it could not draw.

```nvs
<?nvs
use Novis\Pdf\Document;
use Novis\Pdf\Page;

// The logo is a file that the HTML names. `asset` adds it to the document.
bytes $logo = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40\" height=\"20\"><rect width=\"40\" height=\"20\"/></svg>" as bytes;
string $html = "<html lang=\"en\"><head><title>Invoice 1042</title></head><body>"
    . "<img src=\"logo.svg\" alt=\"Shop\"><h1>Invoice 1042</h1><p>Tea: 2.50 EUR</p></body></html>";

// The footer is a function. The render calls it once for each page.
var $rendered = Document::fromHtml($html)
    ->asset('logo.svg', $logo)
    ->footer(fn(Page $page): Core\Html\Markup => html`<p>Page {$page->number} of {$page->count}</p>`)
    ->render();

echo Core\Bytes::startsWith($rendered->bytes, '%PDF-' as bytes) ? 'a PDF file' : 'not a PDF file', "\n";
echo $rendered->pageCount, " page\n";
```
```output
a PDF file
1 page
```
