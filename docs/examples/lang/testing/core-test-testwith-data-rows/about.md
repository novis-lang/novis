A data row gives one test a set of values. You write one row for each set, and each row is reported
as its own case.

`#[TestWith(name: value, …)]` goes above a `#[Test]` method. Each field of the row names one
parameter of that method, and the value is a constant. Repeat `#[TestWith]` for every set of values
you want. The runner calls the method once per row, in the order you wrote the rows, and reports
them as `method#0`, `method#1` and so on. A row that fails is reported on its own, and the rows
after it still run.

Rows and fixtures work together in one test. A parameter that a row names is filled by the row. A
parameter that no row names is filled by the fixture of that type.

**Good to know:** every row of a method fills the same parameters, and a field that names no
parameter does not compile.
