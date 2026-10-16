Returns every key of a `Core\ObjectMap` as a list. The keys are in the order they were added, which
is the same order a `foreach` over the map gives. This replaces a `foreach` over an
`SplObjectStorage` that collects each object into an array.

The list is a new array. If you change the map after the call, the list does not change. If the
map is empty, the result is an empty array.

The key at each position belongs to the value at the same position in the list that `values`
returns.

The examples show listing the keys, the list staying the same when the map changes, and a common
use: printing the names of the users who are online.
