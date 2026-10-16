Makes a `Font` from the bytes of a font file, so that `Image::text` and `Image::measureText` can use it.

`Font::fromBytes` takes the bytes of a TrueType (`.ttf`) or OpenType (`.otf`) file. There are no
system fonts, so your program loads the file itself. Make the `Font` once and use it for many images.

`Font::fromBytes` checks only that the bytes are at least 12 bytes long and start like a font file.
Other bytes throw a `ParseError`. The rest of the file is read when an image is drawn or text is
measured. A font file with broken tables throws a `ParseError` then.

**The examples below** load a font and measure a word, catch bytes that are not a font, and use one
font for several images.
