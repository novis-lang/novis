- **`gaps.py --errors` reads past a `Fault::` call into the next item's doc comment, so one of its
  rows is a phantom.** A `Fault::thrown_as(...)` whose message is `format!`ed above the call carries
  no literal, and the tool's window finds the *following* function's `///` prose instead, listing a
  stem no case can contain. `conformance_coverage.rs` stops its window at a line-leading `///`, so
  the two counts differ by that row; a message built above the call is outside both and cannot be
  matched by its stem. [until: gone tools/gaps.py:--errors]
