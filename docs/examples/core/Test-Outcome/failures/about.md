`Outcome::failures()` returns the failure messages of this run of the test, as an array of strings.
The messages are the same ones the test report prints. When the run passed, the array is empty.

You get an `Outcome` as the parameter of a method with `#[AfterEachTest]`. That method runs after each
test, also when the test failed. You cannot create an `Outcome` yourself.

A test stops at its first failed assertion, and that assertion gives one message. An error the test
throws and does not catch also gives one message, which is the message of the error.

**The examples below** show the failures written to a log file, a count of the messages, and a short
line for a chat message.
