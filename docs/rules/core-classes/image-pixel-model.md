Pixels are **RGBA8**, straight alpha at the API, premultiplied where the maths needs it and never
visible to the caller. One model, and it retires whole families of gd concepts outright.

Palette images go: there is one truecolor model, a GIF is quantised on encode, and a colour is a
value rather than a palette index. Mode flags go, because there is no ambient state to hold them.
Per-pixel access goes, replaced by exporting the whole buffer once. Drawing primitives go entirely —
a rectangle is a blank canvas composited, text is a real font, and charts and diagrams are SVG on the
client; a shape layer is not planned, and a third-party extension may carry one. The affine matrix
members go, replaced by the four cases a web application reaches for, named: resize, rotate, flip,
crop. Screen and window capture go, a screen not being a request's business. Format detection by
positional array and integer constants goes, replaced by a typed header shape and a format enum that
carries its own MIME type and extension.

What this costs is that a program doing genuine raster drawing has no home here. That is deliberate:
the component exists to transform images a web application receives and produces, and every concept
above was priced against that.

**Not shipped.** No image component exists in the tree.
