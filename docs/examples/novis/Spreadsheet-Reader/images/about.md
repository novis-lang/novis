Returns the pictures of one sheet.

`$sheet` is the sheet name, or its position from 0. Each picture has `at`, the cell of its top-left
corner, such as `"C2"`, `image`, the bytes of the picture file, and `format`, its file type, such as
`"png"`. A linked picture is a picture that Excel loads from an address. It has empty `image`
bytes, and `link` is its address. For every other picture, `link` is `null`.

**Good to know:** reading a file never loads a linked picture. The bytes come from the file, so they
are `tainted bytes`. Check the format and the size before you save a picture or show it on a web
page. A CSV file has no pictures, so the list is empty. A sheet that does not exist throws a
`LogicError`. `images` reads xlsx, xlsm and ods files. For an xlsb or xls file, it throws an error.

**The examples below** list the pictures of a sheet, find the picture of each row, and check that the
bytes are the same as the picture that was put in the file.
