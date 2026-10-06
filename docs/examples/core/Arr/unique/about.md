Gives you the entries of an array whose value has not appeared before.

The first entry with a value wins. It keeps its key and its position, and every later entry with the
same value is dropped. The keys are not renumbered, so a list with repeated values comes back with
gaps in its keys.

Two entries are the same when their values are identical: the same type and the same content, with
no conversion. The number 1 and the text "1" are two different values, so both are kept.

The `by` option changes what is compared. It is a function that receives an entry and returns the
value to compare it by. You still get back the whole entry, so you can keep one record per email
address.

The examples show repeated values removed, upper and lower case ignored, and two lists of records
merged without repeating anyone.
