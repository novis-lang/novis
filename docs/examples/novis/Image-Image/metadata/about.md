Keeps the EXIF data of the input file in the output file.

EXIF data is information that a camera or a phone writes into a photo. It can include the date, the
camera model and the place where the photo was taken. By default, `encode` and `variants` remove all
of it, so a photo you publish does not show where a user lives.

`metadata({keep: true})` writes the EXIF data back into the output. This works for JPEG, PNG and WebP
files. An AVIF file never has EXIF data. `metadata({keep: false})` is the same as not calling it.

When `open` turns a photo upright, it also sets the orientation in the kept EXIF data to 1. So the
photo is not turned a second time when somebody opens the file.

**Good to know:** only keep the metadata when you need it, for example for a photo archive. For an
image that strangers will see, the default is the safe choice.
