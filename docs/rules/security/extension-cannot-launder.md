There is **no manifest form** whose declared effect removes `tainted`. The only ways out are a `Core`
member naming the single sink it is safe for (`rule:security/launderers-are-sink-named`) and the
narrow, greppable escape hatch (`rule:security/assert-trusted`), and neither is available to a guest.

A third-party HTML sanitizer can exist as an extension; what it cannot do is *assert* that its output
is safe for HTML. The caller escapes with the `Core` member, or takes responsibility explicitly at the
call site where it is visible and greppable. That is a real capability loss and it is not hidden: the
alternatives are for the sanitizer to be adopted into the standard library, where we own it, or for
the caller to say so out loud. The rejected option is the invisible one.

**On disk.** A manifest key other than `sink` and `source` does not load, nor does a `tainted` written
into a type, so no spelling reaches a parameter mark that launders (`crates/nvs-ext/tests/load.rs`).
