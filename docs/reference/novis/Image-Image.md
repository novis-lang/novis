---
summary: open, resize, crop and convert images, with nothing decoded until the result is written
keywords: image, gd, imagick, imagecreatefromstring, imagecopyresampled, imagejpeg, imagepng, imagewebp, getimagesize, thumbnail, resize, crop, rotate, convert, jpeg, png, webp
---

`Novis\Image\Image` is an image and a list of steps to run on it. Each step method, such as
`resize`, `crop` or `format`, returns a new `Image` and does not change the old one. No pixel is
decoded until `encode`, `variants` or `raw` runs the steps, so bytes that are not an image throw a
`ParseError` only then. `encode` removes all metadata, such as the place a photo was taken, unless
`metadata({keep: true})` is a step.

```nvs
<?nvs
use Novis\Image\Image;
use Novis\Image\Format;

// A PNG of 3 by 2 pixels, written as hex.
bytes $png = Core\Encoding::fromHex(
    "89504e470d0a1a0a0000000d49484452000000030000000208060000009d74661a"
    . "0000000b49444154789c6360c00500001a0001bc3ce0410000000049454e44ae426082"
);

Image $photo = Image::open($png);
bytes $thumb = $photo->resize({width: 6, upscale: true})->format(Format::Jpeg, {quality: 80})->encode();

var $info = Image::info($thumb);
echo Image::mime($info->format), " ", $info->width, "x", $info->height, "\n";
```
```output
image/jpeg 6x4
```
