# Image fixtures

The inputs `crates/nvs-ext/tests/image_decode.rs` and `crates/nvs-ext/tests/image_pipeline.rs` run
through the image component. Each one is a
16 by 12 pixel gradient from red at the top to blue at the bottom, with no metadata, written by
ImageMagick 7 (a free tool) with this command, one run per extension:

```
magick -size 16x12 gradient:red-blue -strip gradient.<ext>
```

`<ext>` is `jpg`, `png`, `webp`, `gif`, `avif` and `jxl`. ImageMagick writes AVIF through libheif and
JPEG XL through libjxl. No file here comes from a photo site, and none carries personal data.

`cmyk.jpg` is 16 by 16 pixels: the top 8 rows are magenta and yellow ink, the bottom 8 rows cyan
and magenta ink, with no metadata and no colour profile. ImageMagick 7 wrote it with this command,
as a YCCK JPEG with an Adobe marker, the way Photoshop stores CMYK:

```
magick -size 16x8 xc:red -size 16x8 xc:blue -append -colorspace CMYK -strip -quality 100 cmyk.jpg
```

Its colour profile is not a file here. `image_pipeline.rs` writes a small CMYK profile of its own,
so no third-party profile and no redistribution licence is involved, and inserts it into
`cmyk.jpg` as an APP2 segment.

`DancingScript-Regular.ttf` is the font the text tests draw in. It is Dancing Script 1.002, a
Latin TrueType font that kerns through both a `kern` table and `GPOS`, copied unchanged from the
`macroquad` crate's examples. It is under the SIL Open Font License 1.1, which
`DancingScript-OFL.txt` beside it holds with the font's copyright.

`NotoSansArabic-Regular.ttf`, `NotoSansHebrew-Regular.ttf` and `NotoSansDevanagari-Regular.ttf` are
the fonts the shaping tests draw in: Arabic joining, right-to-left lines and Devanagari conjuncts.
Each is the unhinted TrueType file from the Noto project's `notofonts.github.io` repository, copied
unchanged. They are under the SIL Open Font License 1.1, which `NotoSans-OFL.txt` holds with the
three fonts' copyrights. No font here is embedded in the component.

`encrypted.pdf` is one page of 200 by 100 points, filled red, encrypted with RC4 at 128 bits. It
has an owner password and no user password, so a reader that is not careful opens it with no
password at all. Ghostscript 10.01 wrote it from a three-line PostScript page with this command:

```
gswin64c -sDEVICE=pdfwrite -sOwnerPassword=owner -dEncryptionR=3 -dKeyLength=128 -dPermissions=-3904 -o encrypted.pdf page.ps
```

`deflate-bomb.pdf` is one page whose content stream is deflated twice: 1,413 bytes that inflate to
512 MiB of spaces. Python's `zlib` wrote it, because the tests have no deflater of their own. Every
other PDF the tests read is built by the test itself.

## Rotated files with a colour profile

`oriented.avif` and `oriented.jxl` store the same 16 by 12 gradient. Each says to rotate the image
90 degrees clockwise to show it, so it shows as 12 by 16. Each embeds a Display P3 colour profile
of 598 bytes.

The profile is moxcms's own Display P3 profile. A small throwaway program built against moxcms
0.8.1 wrote it with `ColorProfile::new_display_p3().encode()` into `p3.icc`. That file is not kept
here, so no third-party profile is involved.

ImageMagick wrote the AVIF with this command:

```
magick -size 16x12 gradient:red-blue -strip -orient RightTop -profile p3.icc oriented.avif
```

The rotation is an `irot` box with angle 3 on the primary item. The profile is a `colr` box of
type `prof`. The file has no EXIF block.

ImageMagick wrote the JPEG XL file losslessly with this command:

```
magick -size 16x12 gradient:red-blue -strip -orient RightTop -profile p3.icc -quality 100 oriented.jxl
```

The codestream is not XYB, and it carries the P3 profile unchanged. ImageMagick does not write the
JPEG XL orientation field, so the field was set afterwards in the image header. The `extra_fields`
flag is on, the orientation is 6, and the tone mapping is the default. The pixel data and the
profile are unchanged. jxl-oxide 0.12.6 reads orientation 6 and the 598-byte profile.

## Files with an EXIF block

`exif.png`, `exif.webp`, `exif.avif` and `exif.jxl` store the gradient with one little-endian EXIF
block of 130 bytes. It has five tags and nothing else: `Make` is `Example`, `Model` is
`Camera One`, `Orientation` is 1, `Software` is `Novis fixtures` and `DateTime` is
`2024:01:02 03:04:05`. There is no GPS data.

A small script wrote the block, and inserted it into the gradient PNG as an `eXIf` chunk.
ImageMagick then wrote the other three files from `exif.png`, one run per extension:

```
magick -size 16x12 gradient:red-blue -strip gradient.png
magick exif.png exif.<ext>
```

`<ext>` is `webp`, `avif` and `jxl`. The WebP file has the block in an `EXIF` chunk. The AVIF file
has it in an `Exif` item, and the JPEG XL file in an `Exif` box. Each of the three puts `Exif` and
two zero bytes in front of the block.

## Animated files

`anim.gif`, `anim.png`, `anim.webp`, `anim.avif` and `anim.jxl` have three frames of 16 by 12
pixels. The first frame is red, the second green (`#00ff00`) and the third blue. Each frame shows
for 100 milliseconds, and the animation loops forever.

ImageMagick wrote the GIF and the lossless WebP file:

```
magick -delay 10 -loop 0 -size 16x12 xc:red xc:#00ff00 xc:blue -strip anim.gif
magick -delay 10 -loop 0 -size 16x12 xc:red xc:#00ff00 xc:blue -strip -define webp:lossless=true anim.webp
```

ffmpeg wrote the other three from the three frames, saved as `f1.png`, `f2.png` and `f3.png`:

```
ffmpeg -framerate 10 -i f%d.png -pix_fmt rgb24 -plays 0 -f apng anim.png
ffmpeg -framerate 10 -i f%d.png -c:v libaom-av1 -crf 10 -cpu-used 6 -pix_fmt yuv444p -loop 0 -f avif anim.avif
ffmpeg -framerate 10 -i f%d.png -c:v libjxl_anim -distance 0 -pix_fmt rgb24 -f rawvideo anim.jxl
```

`anim.png` is an APNG file. `anim.avif` is an AVIF image sequence: a still primary item plus a
track of three frames. `anim.jxl` is a lossless JPEG XL codestream with 10 ticks per second and one
tick per frame. ImageMagick reads only the first frame of `anim.png` and `anim.avif`, and ffprobe
counts three frames in each.

Inputs that are broken on purpose, such as a header declaring more pixels than the cap or an EXIF
block with a malformed IFD, are built by the test itself, so they are not files here. So are the
tagged JPEGs: `image_pipeline.rs` inserts an EXIF block it writes, with an orientation of 6 or a
made-up GPS latitude, into `gradient.jpg`.
