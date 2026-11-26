Returns an attachment with the file name and the content you give. Use it for a file that your
program builds, such as a CSV export.

Pass the result in the `attachments` option of `Core\Mail::send`. The content can be a `string` or
`bytes`. Nothing is written to the disk.

The type is a media type, such as `image/png`. It says what kind of file it is. `fromBytes`
detects it from the first bytes of the content. Plain text has no
such start, so a CSV file or a text file gets `application/octet-stream`. Give the `type` option
for those files, such as `text/csv`.

The file name is checked when `fromBytes` runs. An empty name, a name over 255 bytes and a name
with a control character, such as a line break, throw a `RuntimeError`. A name with letters that
are not ASCII is allowed. The mail encodes it so that every mail program shows it correctly.

**The examples below** send a CSV export, show which type each file gets, and show a file name with
a line break.
