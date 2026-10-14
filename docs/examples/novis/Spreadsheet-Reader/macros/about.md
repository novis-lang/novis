Returns the names of the macro modules in the workbook.

A macro is a small program that Excel can run, written in the language VBA. An xlsm or xls file
can have macros. An xlsx or CSV file has none, so the list is empty. Each name in the
list is a `tainted string`, because it comes from the file.

**Good to know:** reading a file never runs its macros, and `macros` reads only their names, not
their code. Use `macros` to check an upload before you import it. For example, you can reject a
file with macros and ask the person to save it as xlsx. A damaged macro part throws a `ParseError`.

**The examples below** read a workbook without macros, reject an upload that has macros, and check a
CSV file.
