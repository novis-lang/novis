`crates/nvs-cli/src/info.rs` embeds `THIRD-PARTY-LICENSES.txt` and `LICENSE` with `include_str!`, so the
notice and the binary it describes are produced from one tree in one compile and cannot drift. Novis's
own MIT text is embedded for the same reason the third-party texts are: a binary handed to someone
without the repository — an `nvs` installed on a host, or a bundle an author appended their program to
— is still a copy of the software, and MIT asks that its text accompany it.

`nvs info` slices that one embedded file at its two section headings rather than re-formatting it, so
there is exactly one rendering of the component table and it is the one a reader can also open in the
repository. The generator and the reader each carry half of that agreement, and a test fails if either
is changed alone.

The cost is roughly 55 KB of read-only data in a binary measured in tens of megabytes: memory footprint
spent on a legal obligation, never touching a request path.
