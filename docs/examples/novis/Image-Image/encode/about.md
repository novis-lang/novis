Runs every step of the image and returns the bytes of the finished file.

Methods such as `resize`, `crop` and `format` only add a step to a list. `encode` is the call that
does the work. It decodes the input, runs the steps in the order you wrote them, and encodes the
result. All of this happens in one call to the image component.

The output has the format that `format` chose. Without `format`, it has the format of the file you
opened, and a canvas or raw pixels are written as PNG. The output has no EXIF data unless you called
`metadata({keep: true})`.

`encode` throws an error when the input is not an image, when a step cannot run, for example a `crop`
box outside the image, or when the image has more pixels than the limit.

**Good to know:** to write several sizes of one image, `variants` is faster than several `encode`
calls, because it decodes the input only once.
