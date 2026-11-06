---
summary: the records of a CSV file, which `Core\Csv::rows` returns. You read them one at a time with a `foreach` loop
keywords: read CSV file, large CSV file, CSV line by line, stream CSV, CSV records, CSV header, import CSV, low memory
---

`Core\Csv\Rows` contains the records of a CSV file. `Core\Csv::rows` returns it. It reads an open
`Core\IO\File`, an uploaded file, `Core\Request::bodyStream()`, `bytes`, or a list or loop that gives
`bytes` pieces. The class has no methods. You read it with a `foreach` loop.

Each item is one record, as an `array<string>`. The keys are the column numbers, starting at `0`. With
`{ header: true }`, the first record gives the column names, and the keys are these names. The header
is not one of the items.

The file is read while the loop runs, and only one record is in memory at a time. So a file larger
than your memory limit still works. A file is read from its first byte. You can read the records only
once. A second loop continues after the last record the first loop read. After the
last record, a loop gives no items.

`rows` throws a `RuntimeError` when an option is not valid, such as `{ separator: ';;' }`. The loop throws a
`RuntimeError` when the file is closed, and an `IOError` when the operating system cannot read it. It
throws a `ParseError` when a field is not valid UTF-8 text. For a file in another charset, set
`charset`, such as `Core\Charset::Windows1252`. A UTF-8 or UTF-16 byte order mark at the start sets the
charset for you. `rows` does not close the file. Call
`close()` on the file when the loop is done.

```csv file=orders.csv
item,qty
fig,2
apple,10
plum,5
```
```nvs
<?nvs
Core\IO\File $file = Core\IO::open('orders.csv', Core\IO::FileMode::Read);
var $rows = Core\Csv::rows($file, { header: true });

int $total = 0;
foreach ($rows as array<string> $row) {
    echo $row['item'], ': ', $row['qty'], "\n";
    $total = $total + ($row['qty'] as ?int ?? 0);
}
echo 'total: ', $total, "\n";

// The records can be read only once, so this loop gives no items.
int $again = 0;
foreach ($rows as array<string> $row) {
    $again = $again + 1;
}
echo 'second loop: ', $again, "\n";

$file->close();
```
```output
fig: 2
apple: 10
plum: 5
total: 17
second loop: 0
```
