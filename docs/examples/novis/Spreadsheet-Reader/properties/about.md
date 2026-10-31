Returns the document properties of the workbook: the information Excel shows under File, Info.

The result has `title`, `subject`, `author`, `company` and `keywords`, each a `tainted string` or
`null`, and `created`, the time the file was created, as a `Core\Time\Instant` or `null`. An
`Instant` is one moment with no time zone. Call `in` with a zone to get the local date and time.

**Good to know:** a property the file does not have is `null`. Every property of a CSV file is
`null`, because a CSV file is only text. For an ods file, `author` is the person who created the
file. The text comes from the file, so check it before you show it on a web page.

**The examples below** read the title and the author, show the time a file was created in a time
zone, and read the properties of a CSV file.
