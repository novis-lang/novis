Reads compressed data and gives you the original bytes back.

The compressed data can be `bytes`, an open file, an uploaded file, `Core\Request::bodyStream()`, or
a list or loop that gives `bytes` pieces. A file or a stream is read one piece at a time. You pass
the same `Core\Compress::Codec` the data was compressed with. Data that is not in that format throws
a `ParseError`. If the data arrived from outside your program, the result is tainted too.

Every call has a limit on how much it may produce. `$maxBytes` is the largest result you accept,
and `$maxRatio` is the most bytes of result per byte of input. A call that would pass either limit
throws a `ParseError` and returns nothing. You can set smaller limits than the server allows. You
can never set larger ones, and there is no way to switch the limit off.

**In plain words:** a tiny compressed file can unpack into a huge one and fill up a server. These
two limits stop a file that is made to do that.
