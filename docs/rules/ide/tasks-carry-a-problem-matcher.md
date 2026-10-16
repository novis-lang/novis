`nvs run` and `nvs test` are contributed as Tasks, and each carries a `problemMatcher`. Without one the
Tasks print text into a terminal; with one, every diagnostic is a clickable entry in the Problems panel.

It is a two-line regex over the renderer's existing format — `error[E0301]: message`, then
`  --> file:line:col` (`rule:errors/renderings`) — and it is the difference between the Tasks being
useful and being decorative. A failing `nvs test` populating the Problems panel through the matcher is
part of the extension-host run.
