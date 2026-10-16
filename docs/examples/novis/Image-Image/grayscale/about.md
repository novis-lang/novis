Removes the colours of an image and keeps how light or dark each pixel is.

Every pixel gets the same red, green and blue value. The value is the light of the pixel as a person
sees it, so green counts more than red, and red counts more than blue. Transparency does not change.

`grayscale` returns a new `Image` and does not change the one you called it on. The work happens
when `encode`, `variants` or `raw` runs.

**Good to know:** to give a gray image one colour, for example an old brown photo, call `tint`
after `grayscale`.
