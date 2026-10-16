`Core\IO::writeStream(string $path, Iterable<bytes> $src, {max?, overwrite?})` is where a stream
reaches disk, and every convenience that writes one delegates to it. Path first, because the subject
is parameter one and the plain write already reads that way.

It is an ordinary `Core\IO` member and therefore an ordinary **path sink** requiring `fs.write`, with
no exemption for arriving by way of an upload: a `tainted` filename reaching it is a compile error
exactly as it is everywhere else, and the containment check is the launderer.

Two rules are its own. **`overwrite` defaults to false**, because a destination chosen from a
client's claimed filename is the case this member exists to serve. **A write that fails mid-stream
removes the partial file**, because a truncated file the application believes it wrote is a worse
failure than an error — the caller learns from the throw, not from a later reader.

It is general on purpose: a request body, a decompressed archive, an outbound response body and an
upload part all reach disk through this one implementation, so the partial-write cleanup lives in one
place rather than in every call site that hand-wrote the loop.
