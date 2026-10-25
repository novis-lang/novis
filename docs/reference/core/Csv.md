---
summary: RFC 4180 documents read into rows of `string` fields and written back, with an optional header row and dialect
keywords: CSV, comma-separated, parse CSV, write CSV, RFC 4180, header row, delimiter, separator, quote, dialect
---

`Core\Csv::parse` reads a whole document into `array<array<string>>` — every field is a `string`,
quoted fields and embedded newlines included. With `{ header: true }` the first record is consumed
as column names and every row is keyed by them; without it a row's keys are `"0"`, `"1"`, ….
`format` writes each row by its values in order (its keys are ignored), quotes a field only where a
byte would change the parse, and `{ header: [...] }` writes a first record. A field can be a
`tainted string`, such as a value read from a database, and then the document is a
`tainted string` too. The dialect options
`separator`, `quote` and `escape` are single ASCII bytes.

```nvs
<?nvs
string $doc = "name,qty,note\nfig,2,ripe\nplum,10,\"a, b\"\n";
var $rows = Core\Csv::parse($doc, { header: true });
echo Core\Arr::count($rows), " rows\n";
foreach ($rows as array<string> $row) {
    echo $row['name'], '=', $row['qty'], ' (', $row['note'], ")\n";
}

var $plain = Core\Csv::parse("a;b\n1;2\n", { separator: ';' });
echo $plain['1']['0'], '+', $plain['1']['1'], "\n";

array<array<string>> $out = [['fig', '2'], ['plum', 'a, b']];
echo Core\Csv::format($out, { header: ['item', 'note'] });
```
```output
2 rows
fig=2 (ripe)
plum=10 (a, b)
1+2
item,note
fig,2
plum,"a, b"
```
