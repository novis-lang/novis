Runs every step of the image and returns the bytes of the finished file.

Methods such as `resize`, `crop` and `format` only add a step to a list. `encode` is the call that
does the work. It decodes the input, runs the steps in the order you wrote them, and encodes the
result.

The output has the format that `format` chose. Without `format`, it has the format of the file you
opened. An image made with `Image::create` or `Image::fromRaw` is written as PNG. The output has no
EXIF data (the camera and location details in a photo) unless you called `metadata({ keep: true })`.

The result is `tainted bytes`, because the image may be an upload. Write `tainted bytes` on the
variable that keeps it. A response body, a file and `Core\Storage` all take `tainted bytes`.

`encode` throws an error when the input is not an image. It also throws when a step cannot run,
such as a `crop` box outside the image. An image with more pixels than the limit throws too.

**Good to know:** to write several sizes of one image, `variants` is faster than several `encode`
calls, because it decodes the input only once.
