An assertion that fails throws `Core\Test\Failure`.

Every `Core\Test::assert…` method either returns and lets your code carry on, or throws. The message
names the method that failed and the values it compared, so you can read what went wrong without
opening the file again. The `message` option puts your own sentence in front of that text.

`Core\Test\Failure` is not a `RuntimeError`. It sits directly under `Throwable`, so a `catch
(RuntimeError $e)` in the code you are testing never catches it by accident. A `catch (Throwable $e)`
does catch it.

Inside a `#[Test]` method the test runner catches the failure and reports that test as failed.
Outside one it is an ordinary throw: catch it, or it ends your program.

**The examples below** show the message, the `catch` arm that takes it, and a check outside a test.
