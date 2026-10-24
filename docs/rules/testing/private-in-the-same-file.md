A test method may reach `private` and `protected` members of classes declared **in the same file**.
A test in a separate file is held to the public contract.

This is the narrowest rule that avoids the failure it exists to prevent: a `public` method that
exists only because a test needed to reach it. White-box testing stays possible where the author has
already chosen to put the test next to the code; everything at a distance tests behaviour.

**A test method is test code** in `rule:testing/tests-never-reach-a-build`'s sense: a method that
carries `#[Test]`, `#[Fixture]`, `#[BeforeClass]`, `#[AfterClass]` or `#[AfterEachTest]`, and every
anonymous function written inside one. A fixture builds what a test reaches, so it is admitted on the
same terms. Any other method of a test class is held to the ordinary levels, so a helper that reads a
`private` member has to be test code itself.

**The file is the one that declares the member's class**, read off that class's declaration. A test
class that `extends` the class from another file reaches its `protected` members the ordinary way and
nothing more.

**It costs nothing on the request path.** The admission is decided at compile time, in
`crates/nvs-types/src/signatures.rs`'s `is_visible_here`, and test code never reaches a build, so no
request can call a method that used it.

**Declined:** an attribute such as `#[VisibleForTesting]` on the member. It moves the decision to
the class under test, and the member is then reachable from every test anywhere.
