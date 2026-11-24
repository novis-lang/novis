Puts the image `$data` at the cell `$cell`, and returns the sheet. The image is a PNG, JPEG or GIF
file, read from its bytes.

`width` and `height` are the size of the image in pixels. Without them, the image keeps its own
size. `offsetX` and `offsetY` move the image right and down from the top left corner of the cell, in
pixels.

**Good to know:** any other format throws a `LogicError`, so check an image from an upload before
you add it. A file with bytes missing at the end also throws. So does an image that ends after the last column
(`XFD`) or the last row (1,048,576). An image added after the first row stays in memory until the
sheet is finished. To use less memory, add many images before the first row. The same image added
again is stored in the file once. A CSV file has no images, so `addImage` does nothing there.

**The examples below** put a logo at the top of a report, and add a picture for each product. The
last one shows the error for a file that is not an image.
