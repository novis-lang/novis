Puts the image `$data` at the cell `$cell`, and returns the sheet. The image is a PNG, JPEG or GIF
file, read from its bytes.

`width` and `height` are the size of the image in pixels. Without them, the image keeps its own size.
`offsetX` and `offsetY` move the image right and down from the top left corner of the cell, in
pixels.

**Good to know:** any other format throws a `LogicError`, so check an image from an upload before you
add it. A file that is cut short also throws a `LogicError`. So does an image that ends after the
last column (`XFD`) or the last row (1,048,576) of the sheet. An image added before the first row of the sheet is written at once. An image added later is
kept in memory until the sheet is finished, so a report with many images adds them before its rows.
A CSV file has no images, so `image` does nothing there.

**The examples below** put a logo at the top of a report, add a picture for each product, and show
the error for a file that is not an image.
