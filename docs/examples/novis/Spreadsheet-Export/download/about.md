Sends the spreadsheet to the browser as a file to download.

`$name` is the file name the browser shows. It also sets the format: a name that ends in `.csv`
makes a CSV file, and any other name makes an Excel file (xlsx). The file is sent in parts while it
is written, so a large export does not wait until every row is ready. `download` returns nothing.

**Good to know:** a name with characters outside plain ASCII, such as `März.csv`, is sent in a form
every browser reads. A quote or a line break in the name cannot add a header to the response.
