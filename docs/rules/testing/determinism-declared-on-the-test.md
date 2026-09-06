`#[Test(at: "2026-01-01T00:00:00Z", seed: 42)]` fixes the clock and the random generator of that
test's own isolate: `Core\Time::now()` answers exactly that instant, and the seeded generator and
the identifier members draw the same sequence on every run, every OS and every architecture.
`Core\Test::advance(Duration)` moves the fixed clock forward, and refuses when no clock was fixed —
advancing a real clock is not something a test can ask for. An `at:` that is not an RFC 3339
timestamp fails that test where it is declared.

Nothing holds state behind a function's back here. The clock is *isolate configuration*, declared at
the test where it is visible and inert everywhere else, in the same category as a time zone set in
`nvs.toml` — and a test declaration reaches no built artifact, so none of this exists in production
even in principle.
