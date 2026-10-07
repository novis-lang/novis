How `resize` fits an image into the width and height you give it.

You pass a `Fit` to `resize` as `fit`. It decides what happens when the image and the box you give
have a different shape:

- `Cover` fills the whole box and cuts off what does not fit. This is the default.
- `Contain` keeps the whole image inside the box. The result is exactly the box size, and the empty
  part is transparent.
- `Fill` stretches the image to the box size. The shape of the image changes.
- `Inside` keeps the whole image, and the result is no larger than the box. One side may be smaller.
- `Outside` covers the box without cutting anything off. One side may be larger than the box.

With `Cover`, `gravity` chooses which part of the image is kept.
