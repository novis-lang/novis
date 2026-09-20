Gives you the entries of an array whose value has not appeared before.

The first entry with a value wins. It keeps its own key and its own position, and every later entry
with the same value is dropped. The keys are not renumbered, so a list with repeated values comes back
with gaps in its keys.

Two entries are the same when their values are identical: the same type and the same content, with no
conversion. The number 1 and the text "1" are two different values, so both are kept. PHP's
`array_unique` compares the text of each value instead, and collapses them into one.

The `by` option changes what is compared. It is a function that receives an entry and returns the
value to compare it by. The entry you get back is still the whole entry, never what the function
returned, so you can keep one record per email address or one order per customer.

The examples show how to remove repeated values, how to ignore upper and lower case, and how to merge
two lists of records without repeating anyone.
