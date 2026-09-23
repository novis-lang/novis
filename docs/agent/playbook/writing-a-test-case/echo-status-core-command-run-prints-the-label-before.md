- **`echo "status: ", Core\Command::run()` prints the label before whatever the callee echoes.**
  `echo`'s arguments are written as they are evaluated, so a member that produces output inside the
  call interleaves with the text around it and the expectation reads `status: greet: Hello...`,
  which looks like a matcher bug. Take the value into a variable first; every dispatching or
  callback-taking member has this shape. [until: reviewed 2026-09-06]
