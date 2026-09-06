A test method may reach `private` and `protected` members of classes declared **in the same file**.
A test in a separate file is held to the public contract.

This is the narrowest rule that avoids the failure it exists to prevent: a `public` method that
exists only because a test needed to reach it. White-box testing stays possible where the author has
already chosen to put the test next to the code; everything at a distance tests behaviour.
