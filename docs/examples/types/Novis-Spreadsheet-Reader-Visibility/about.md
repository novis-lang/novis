Whether a sheet of a workbook shows its tab in Excel.

- `Visible` is a normal sheet.
- `Hidden` is a sheet without a tab. A person can show it again in Excel.
- `VeryHidden` is a sheet that only a program can show again.

When you write a workbook, the sheet option `hidden` sets it. `hidden: true` is `Hidden`, and
`hidden: 'very'` is `VeryHidden`. When you read a file, `Reader::sheets` gives the `visibility` of
each sheet.

**Good to know:** a hidden sheet is not secret. Any program that reads the file can read its cells.
