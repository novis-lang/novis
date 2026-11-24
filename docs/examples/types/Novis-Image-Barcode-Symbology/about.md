The barcode format that `Barcode::render` and `Barcode::svg` draw. A barcode format is also called a
symbology.

`Barcode::Symbology` is declared in the class `Novis\Image\Barcode`.

Each format takes different data. The data must already contain its check digit.

- `Code128` takes any plain ASCII text, such as a tracking number.
- `Gs1128` takes GS1 element strings. Each one starts with its identifier in brackets, as in
  `(01)09501101530003(10)ABC`.
- `Ean13` takes the 13 digits printed on a product.
- `Ean8` takes 8 digits, for small products.
- `UpcA` takes the 12 digits used on products in North America.
- `Code39` takes digits, capital letters, spaces and the characters `- . $ / + %`.
- `Itf14` takes the 14 digits printed on a shipping box.
- `DataMatrix` takes any text and draws a square code.

The others draw lines of bars. Data that the format cannot contain throws a `LogicError`.
