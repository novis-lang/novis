Beyond the pipeline, each remaining job is exactly one entry point.

**Comparison** decodes both inputs, refuses a size mismatch, and answers whether they are identical,
how many pixels differ, the maximum delta, a structural similarity score, and optionally a rendered
diff image for a test report. There is no assertion member: a composite assertion is an ordinary
method, so a test asserts on the comparison's own numbers and the failure ledger records it.
**Perceptual, difference and average hashing** plus a distance function serve near-duplicate
detection. **Placeholders** and **palette extraction** serve the front end.

**Text** renders over a font supplied as bytes, with no system font lookup, since the guest has no
filesystem. **QR codes** are one entry point, because a QR code is an image a web application
produces daily. **SVG** is an input format whose declared canvas counts against the pixel cap, whose
scripts and foreign objects are ignored, and where **no external reference is ever resolved** —
there is nothing in the guest to resolve it with. Rasterising is how an uploaded SVG is displayed
safely; sanitising one would be the repair that is refused everywhere else.

**Not shipped.** No image component exists in the tree.
