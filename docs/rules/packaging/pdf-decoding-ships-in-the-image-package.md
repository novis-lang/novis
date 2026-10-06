PDF joins the image component's format roster **decode only**, in the second wave beside SVG, with
`Format` gaining a `Pdf` case and no new entry point: the plan/terminal shape and the component's closed
set of exports are unchanged (`rule:core-classes/image-format-roster`,
`rule:core-classes/pdf-page-is-an-image-source`).

It is not a member of `nvs/pdf`. A writer and an interpreter share no code, and a raster produced in the
generation package would have to recross the boundary to enter the image pipeline that is the whole
point of loading one. Generation stays `rule:core-classes/pdf-render-has-no-io`'s; text extraction,
page manipulation and forms are different jobs and stay in the third-party channel.

The usual alternative is ImageMagick delegating to Ghostscript — an installed, unsandboxed
interpreter with an RCE history long enough that ImageMagick's stock policy ships with the PDF coder
disabled. A PDF interpreter is a strictly larger hostile-bytes case than any format already on the
roster, and the sandbox is where a parser that size belongs.
