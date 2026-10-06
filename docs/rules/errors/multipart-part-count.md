`[limits] max_multipart_parts` defaults to **1000**, beside a per-part field-name length and a
per-part size.  Exceeding any of them is a `413` naming both the limit and the directive.

The body-size cap is not this. The cost of a part is bookkeeping and a temp file, not bytes, so a
body well inside every size cap can still exhaust inodes, a known denial-of-service shape in
multipart parsers, and 1000 is the number that shape's best-known fix chose.

This also widens the attribution rule in the only direction it was missing: **temp files, file
descriptors and disk bytes are charged to a request the same way memory is.**
