Returns every value that a route captured from the path, as an array. Each key is the name in the
route's path, such as `id` for `{id}`. The values are in the order of the path.

Each value is already percent-decoded, so `%20` in the path is a space. Each value also has the
type of its handler parameter: `{id}` read by `uint $id` is a number. A text capture is `tainted`,
because the visitor chose it.

An optional capture that the path does not have is not in the array. A route with no captures
returns an empty array.

The examples show every capture of a path, routes with no captures, and a log line that lists
what a request asked for.
