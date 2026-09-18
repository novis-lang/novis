A native extension is a compiled library the owner of a machine adds to Novis, and this is where
they say which ones there are and where each one lives.

An extension brings new classes with it — an image codec, a driver, something one deployment needs
that the language does not ship. Because it is native code running inside every request on the
host, only the account that owns the configuration file may name one, and every entry carries the
checksum its file is required to have. A binary that changed underneath its checksum is refused,
and the refusal takes the whole set rather than loading the rest of it.

The set can change on a running server. Nothing compiled against the old one is ever reused against
the new one, so an extension is added, replaced or removed without dropping a request. An
application can neither add one nor read the list: what it sees is the classes.
