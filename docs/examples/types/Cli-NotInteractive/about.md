The error a question throws when nobody is there to answer it.

A command-line program can ask a question, such as a name, a yes or no, or a choice from a list.
It can also run where no person is watching, such as a nightly job, a build step or a queue worker.
Novis does not wait there for an answer that never comes. A question with a `default` returns that
value and the program goes on. A question without a `default` throws this error at once.

So you decide what happens. A program that should run without a person gives every question a
`default`. A program that must not guess catches this error and stops, and says what it needed.
For example, a program that deletes data, sends messages or charges money must not guess.

**Good to know:** a question is shown on the terminal the program was started from, not read from its
standard input. So a program that reads piped data can still ask a question.

**The examples below** show a question nobody answers, the same question with a `default`, and a
release script that stops when nobody answers.
