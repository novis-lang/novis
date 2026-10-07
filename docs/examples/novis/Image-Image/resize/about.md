Changes the size of an image.

Give only `width` or only `height`, and the other side follows, so the image keeps its shape. Give
both, and `fit` decides how the image fills that box. `Fit::Cover` is the default. It fills the
whole box and cuts off what does not fit. `Fit::Contain` shows the whole image inside the box and
leaves the rest transparent. `Fit::Inside` keeps the shape and makes the image no larger than the
box.

`resize` returns a new `Image` and does not change the one you called it on. The work happens
when `encode`, `variants` or `raw` runs.

**Good to know:** `resize` never makes an image larger than it is. Set `upscale` to `true` when you
want that.
