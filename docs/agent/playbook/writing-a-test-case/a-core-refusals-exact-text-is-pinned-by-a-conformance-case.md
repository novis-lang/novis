- **A `Core` refusal's exact text is pinned by a conformance case, so `cargo test` stays green while
  `verify.py` fails at step 7 on a reworded message.** The `.nvst` expectation holds the whole
  sentence, and changing only the advice half of one is enough to fail it. Grep
  `tests/conformance/` for a distinctive phrase of the message before you reword it rather than
  after the verify run. [until: reviewed 2026-09-13]
