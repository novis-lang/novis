The constructor is the test's setup, and a stronger one than a `setUp` method can be: definite
property initialization guarantees every property is assigned before any test body runs. There is no
`#[Before]`, and no all-cases pair either — across isolates those would either lie or need fixtures,
which is the honest spelling of what they were for.

A test class's constructor therefore declares no parameters. One that does is reported as that test
failing rather than pretended past: the runner constructs the class with no arguments, and a test
method's own parameters are filled by `rule:testing/fixtures` and `rule:testing/data-rows` instead.
