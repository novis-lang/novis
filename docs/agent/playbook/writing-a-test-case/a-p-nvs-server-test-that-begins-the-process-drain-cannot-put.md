- **A `-p nvs-server` test that begins the *process* drain cannot put it back, and every test in the
  binary shares that bit.** `Drain::process` hands out a handle on one `OnceLock` atomic and `begin`
  is the only writer there is, so a case asserting that a server is not draining races the case that
  began it, whichever order cargo runs them in. `is_draining_answers_the_same_on_every_core` is the
  one case here that begins it and its doc comment says so; write every other one against
  `Draining::detached()`, which is what that constructor exists for. [until: reviewed 2026-09-09]
