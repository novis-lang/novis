`nvs pkg audit` lists each version in your `package.lock` that has an advisory. An advisory is a
retraction or a withhold of a version. For each one, the command prints the reason and the smallest
version that fixes it. It exits with an error while a locked version has an advisory, so you can run
it in CI. `--json` prints the same report as JSON.

The registry publishes its advisories in a signed file. Every advisory is also an entry in the
registry's transparency log, and the command compares the two. A registry cannot hide an advisory
without the command stopping with an error. When the registry names a second copy of its log's
checkpoint on another host, the command checks that both copies show the same log.

**Good to know:** the command also warns about code in a namespace that no line of `[grants]`
matches. That code has every permission of your application. A warning does not change the exit
code.
