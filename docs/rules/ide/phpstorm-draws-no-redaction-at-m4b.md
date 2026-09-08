`nvs/redactions` is a request on the one server both clients drive, so the PhpStorm plugin can answer it
whenever it is built — the range computation is shared, per
`rule:ide/redaction-ranges-come-from-the-server`. What is not shared is the drawing: an editor-side
decoration is per-editor work with no common half.

The plugin is not built at M4B, so PhpStorm conceals nothing at this milestone. That is a decision rather
than something discovered when someone opens a `.nvs` file in PhpStorm on a call, and the PhpStorm side is
due when its plugin is.
