Returns every line of one header of a reply, as an array of strings.

A server can send the same header on more than one line. `headers()` gives one string for each
line, in the order the lines arrived. A header that arrived once gives an array with one string,
and a missing header gives an empty array. The name does not depend on upper or lower case. Each
value comes from another server, so it is `tainted` until your program checks it.

Use `headers()` to read cookies, because `header()` throws an error for `Set-Cookie`. Use it also
when each line has its own meaning, for example a `Link` header.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They read every cookie of a reply, show a missing header, and find the next page
of a list in the `Link` headers.
