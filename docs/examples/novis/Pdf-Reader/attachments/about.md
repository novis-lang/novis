Returns the files attached to a PDF file.

`attachments` returns a list. Each item has a `name`, which is the file name the PDF gives, and
`data`, which is the content as bytes. A PDF file with no attached files gives an empty list. An
electronic invoice has its XML in this list too, with the name `factur-x.xml`.

The name comes from the sender of the file. Do not use it as a path on your server without a check.

**The examples below** list the files of a received invoice, show the empty list of a letter, and
find one file by its name.
