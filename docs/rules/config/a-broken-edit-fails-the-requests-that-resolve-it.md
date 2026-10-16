When a revalidated file no longer compiles, the path's pointer is left where it was — it still names
the last content that compiled — but the `Failed` state the resolution just reached is what **this**
caller gets, as ordinary checked-return data. A later request landing on the same content sees the
same `Failed` entry, because it is the same key, and is answered from the table rather than compiled
again, so a request storm against a broken file costs one compile and one rendering of its spans, not
one per request.

Requests already running are unaffected (`rule:config/a-request-keeps-the-unit-it-resolved`); only
requests that newly resolve the broken file fail, and they fail loudly. Silently continuing to serve
the last good version after an edit — especially a security fix — is the worse failure mode, and it
would buy availability nothing needs.

The same policy covers an extension removed while source still references it: nothing proves that at
reload time, and the units that call it fail when a request next resolves them, and only those.
