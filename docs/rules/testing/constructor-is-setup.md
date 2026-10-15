The constructor is the test's setup, and a stronger one than a `setUp` method can be: definite
property initialization guarantees every property is assigned before any test body runs. There is no
`#[BeforeEachTest]`, because the constructor is that hook. Cleanup after each test is
`rule:testing/after-each-test`, and setup and cleanup done once around a whole class are
`rule:testing/class-hooks`.

A test class's constructor therefore declares no parameters. One that does is reported as that test
failing rather than pretended past: the runner constructs the class with no arguments, and a test
method's own parameters are filled by `rule:testing/fixtures` and `rule:testing/data-rows` instead.
