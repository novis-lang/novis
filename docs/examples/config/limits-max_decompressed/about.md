The most any one decompression may produce.

A compressed archive says how large it unpacks to, and a hostile one lies: a few hundred octets on
the wire can become gigabytes in a heap. This is the absolute half of the bound every `Core\Compress`
and `Core\Zip` decode runs under — a ceiling on the output, applied while that output grows rather
than to a buffer sized from the archive's own header, so a bomb costs the ceiling and never the size
it claimed.

A call may name a smaller ceiling of its own and gets the smaller of the two; nothing raises this
one, and there is no spelling for an unbounded decompression at all. Passing the bound throws rather
than truncating, because a truncated decompression that looks like success is the failure the bound
exists to prevent. Written nowhere, it is 64 MiB — large enough that no honest document reaches it.

The example prints the ceiling in force, works a decode against both sides of a smaller one, and asks
to raise the operator's.
