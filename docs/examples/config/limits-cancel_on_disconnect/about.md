The request methods that stop as soon as their client goes away.

A client can close its connection before it gets the answer. By default the request still runs to
its end, and the server throws the answer away. A request whose method is in this list stops at
once instead. It runs no more code of your program, and no `catch` block runs.

The default is `[]`, an empty list. A list such as `["GET", "HEAD"]` suits a site where these
methods only read data. The names are compared exactly, so `GET` and `get` are two different
methods. Only the person who runs the server sets this list. A request cannot change it.

**The example below** tries to change the list from inside a request.
