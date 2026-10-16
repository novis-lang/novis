Makes an image soft, so edges and small details are no longer sharp.

`$sigma` is the strength of the blur, in pixels. A larger value mixes each pixel with pixels that are
further away. `blur` is useful for a background behind text, or to hide details in a preview.

`blur` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.

**Good to know:** a strong blur on a large image takes longer. Make the image smaller with
`resize` first when the result does not need all of its pixels.
