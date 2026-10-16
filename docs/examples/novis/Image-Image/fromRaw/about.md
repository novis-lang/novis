Starts an image from pixels in memory, in the same shape that `raw` returns.

The argument has a `width`, a `height` and `pixels`. `pixels` contains the image row by row, from the
top-left corner. Each pixel is four bytes: red, green, blue and alpha, each from 0 to 255. This is
called RGBA8.

The length of `pixels` must be exactly `width` times `height` times 4. Otherwise `encode`, `variants`
or `raw` throws an error. An image with more pixels than the server's limit also throws an error.

**Good to know:** together with `raw`, this lets you change single pixels in Novis code and then
encode the result. For changes to the whole image, the step methods such as `resize` are faster.
