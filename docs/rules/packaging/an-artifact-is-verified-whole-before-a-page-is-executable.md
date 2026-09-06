A reader `mmap`s the file read-only, never starting from an executable mapping. It checks `magic`,
`format_version` and `env_hash` against what this process expects, checks `payload_len` against the
mapping's own length, and computes BLAKE3 over the mapped payload bytes against the header's checksum.
Every mismatch is a cache miss (`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`), and not
one byte of the payload is touched before the checksum matches.

Then **place, relocate, protect, bind.** The reader maps a second region of its own — private,
anonymous, writable — and copies each allocatable section into it at the alignment that section runs
at. It walks the object's relocations and resolves every undefined symbol against this process's own
addresses: a runtime helper's, or the `ClassDesc` this process built from the IR for the class the
symbol names. Only after that are the pages `mprotect`'d to `PROT_READ | PROT_EXEC` — a mapping is
writable or executable, never both, and the transition runs one way. Descriptors are built before
relocation and their method rows are filled with code addresses after protection, writing into the
descriptors and never back into the pages.

**The pages that run are the reader's, never the file's.** The file's mapping is never writable, so
nothing can reach the bytes the checksum covered and no relocation can launder a corrupt payload past
the check. Owning the layout also lets the reader place a landing area beside the code: a symbol
further away than a PC-relative call can reach gets an eight-byte cell holding its full address and a
jump through it, so every displacement written names a target inside the reader's own mapping. A
relocation that still does not fit is a miss, never a truncated address; so is an unresolvable symbol.
