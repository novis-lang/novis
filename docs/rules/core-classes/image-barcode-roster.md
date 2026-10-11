`Novis\Image\Barcode` renders **Code 128, GS1-128, EAN-13, EAN-8, UPC-A, Code 39, ITF-14 and
DataMatrix**, chosen by `Barcode::Symbology`, each as an SVG string or as any raster format the
component encodes. QR codes stay `Novis\Image\QrCode`'s, which gains SVG output too.

| Member | What it does |
|---|---|
| `Barcode::render(string $data, Barcode::Symbology $symbology, ?{height?: uint, module?: uint, margin?: uint, text?: bool, format?: Format} $options = null): bytes` | the code as an encoded image, PNG by default |
| `Barcode::svg(string $data, Barcode::Symbology $symbology, ?{height?: uint, module?: uint, margin?: uint, text?: bool} $options = null): string` | the code as an SVG document |
| `QrCode::svg(string $data, ?{margin?: uint, level?: QrCode::Level} $options = null): string` | a QR code as an SVG document |
| `QrCode::giroCode({name: string, iban: string, amount?: decimal, bic?: string, reference?: string, text?: string}): string` | the EPC payment text (GiroCode) a banking app reads, for `QrCode::render` or `QrCode::svg` |

A wrong check digit, or a character the symbology cannot carry, throws `LogicError` naming it; a code
is never repaired by computing a digit the program did not write. `giroCode` checks the IBAN's checksum
and the field lengths the EPC layout fixes, and is Novis source: the payload is text, so it needs no
crossing. SVG output is vector, so a code in a PDF prints sharp at any size; it is the one form a PDF
document needs (`rule:core-classes/pdf-one-engine`).

The 1D codes are `barcoders` and DataMatrix is `datamatrix`, both pure Rust in the image component's
sandbox. Every code a test writes is read back by a reader that is a dev-dependency of the test only.

- **Memory:** a code is at most a few thousand modules, so a render is far under the pixel cap.

**Shipped.** The `barcode` export draws every symbology, `qr` writes `svg`, and `Barcode`,
`QrCode::svg` and `QrCode::giroCode` are in the binary. `extensions/image/src/barcode.rs` says what
each symbology takes and what each option's default is. `crates/nvs-ext/tests/image_barcode.rs`
reads every code back with `rxing`, and `crates/nvs-cli/tests/image_barcode.rs` pins the GiroCode
layout.

**Declined:**
- **PDF417 and Aztec.** Each is a large encoder for documents almost no web application produces.
- **A barcode reader.** Reading a code from a photo is a computer-vision job, outside a codec
  component.
- **Computing a missing check digit.** The program wrote a number; the code carries that number or
  throws.
