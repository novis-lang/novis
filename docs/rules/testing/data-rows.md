`#[TestWith(...)]` carries an anonymous object that the checker matches against the method's parameters
**by name and by type**. It is the one test marker that may repeat on a declaration, and each row is
its own separately reported case, labelled `method#N` in the order the rows are written — so a row
that fails is reported alone rather than stopping the rows after it, and a skip is stated per row,
a row being a test.

Every way a row and a parameter list fail to line up is one refusal, because the fix is the same
every time: write the row against the parameter list. A field naming no parameter, a value that is
not a literal of that parameter's declared type, a row omitting a field its siblings supply, and the
marker written on a method that is no test at all.

Where a row and a fixture could both answer one parameter, the **row** wins: it was written against
this method's own parameter list, while a fixture answers every method of the class at once. Each
row's values are folded to constants in parameter order rather than in written order, because that
is the order the call is made in, and are released when that call returns.
