Draws another image on top of this one.

The image on top is called the overlay. It is a whole `Image` of its own, so it can have its own
steps, for example `resize` or `grayscale`. Those steps run first, and then the overlay is drawn.

`gravity` places the overlay, for example `Gravity::SouthEast` for the bottom-right corner. The
default is `Gravity::Center`. `x` and `y` give the position of the overlay's top-left corner in
pixels instead. They may be negative. The parts of the overlay that are outside the image are cut
off, and the image keeps its size.

`opacity` makes the overlay partly transparent. It is from 0.0 to 1.0, and the default is 1.0.
`blend` chooses how the colours are mixed. The default is `Blend::Normal`, where the overlay covers
the image. `Blend::Multiply` makes the result darker, and `Blend::Screen` makes it lighter.

`composite` returns a new `Image` and does not change the one you called it on. The work happens
when `encode`, `variants` or `raw` runs.

**Good to know:** a `format` or `metadata` step on the overlay changes nothing. The result is
written in the format of the image below.
