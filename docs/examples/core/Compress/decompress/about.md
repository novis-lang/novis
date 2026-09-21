Reads compressed data and gives you the original bytes back.

You pass the same `Core\Codec` the data was compressed with. Nothing is guessed, so data that is
not that format throws a `ParseError`. If the data arrived from outside your program, the result
stays tainted: unpacking bytes tells you nothing about what they are safe for.

Every call has a limit on how much it may produce. `$maxBytes` is the largest result you accept,
and `$maxRatio` is the most bytes of result per byte of input. A call that would pass either limit
throws a `ParseError` and returns nothing at all. You can ask for smaller limits than the server
allows. You can never ask for larger ones, and there is no way to switch the limit off.

This replaces PHP's `gzdecode`, `gzuncompress`, `gzinflate` and `zlib_decode`, where the length
limit was optional and unlimited by default.

**In plain words:** a tiny compressed file can unpack into a huge one and fill up a server. These
two limits are what stop a file built to do that.
