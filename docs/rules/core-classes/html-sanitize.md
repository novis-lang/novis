`Core\Html::sanitize` answers `Core\Html\Markup`, which makes it the fourth way to obtain one and the
only one that takes a runtime-computed string.

That is exactly why it must be a parser that **rebuilds the document from a known-good grammar**, and
never a filter that deletes what looks dangerous. A filter answering a carrier would be a generic
sanitizer wearing a type — it would claim a guarantee it cannot establish, because "what looks
dangerous" is a list an attacker gets to extend.

**Not shipped.** `crates/nvs-stdlib/src/html.rs` carries `escape` and `toSource` and no sanitizer.
The member waits on the WHATWG tree (`rule:core-classes/html-parsing`), which is what it would parse
into.
