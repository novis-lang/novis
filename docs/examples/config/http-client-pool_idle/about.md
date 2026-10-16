How many outbound connections one core keeps open between calls, ready for the next one.

Opening a connection to an upstream API costs a round trip, and a TLS handshake on top of that, so a
connection that has finished its call is kept rather than closed. The ones being kept are idle: they
hold a socket and a buffer and are doing no work. This is how many of them a single core will hold
on to, which makes it a decision about the size of the machine — a host with sixteen cores holds up
to sixteen times this number.

**In plain words:** it is the size of the shelf that finished connections are put back on, per core.
A bigger shelf means fewer handshakes and more memory sitting idle; `0` means the shelf is taken
away and every call opens its own connection.

Because the shelf belongs to the core and outlives the request that filled it, a program may read
this number and never raise it. A request that could would be spending memory that every other
request on that core then goes without.
