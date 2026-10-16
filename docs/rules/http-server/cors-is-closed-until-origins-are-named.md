With `[http.cors] origins = []`, which is the shipped value, no CORS header is emitted at all and
a preflight is answered `403`. That is what closed means, and it is the correct default because
a browser's same-origin policy is doing its job until somebody deliberately relaxes it.

Naming an origin opens exactly that origin: a matching `Origin` is echoed back and the answer
varies on it, a non-matching one gets nothing, and a preflight from a named origin is granted with
what the block configures — `methods` (shipped `GET`, `HEAD`, `POST`), `headers`, `expose`,
`credentials` and `max_age`. An origin is matched exactly and never normalised. `["*"]` answers
everybody without varying, and is permitted only beside `credentials = false`
(`rule:http-server/a-wildcard-origin-with-credentials-is-refused`).

A plain `OPTIONS` and a cross-origin `GET` are not preflights and are handed to the route. A
route that needs its own policy sets it for itself, request-locally
(`rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`).
