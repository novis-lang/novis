`<?= $value ?>` is short for `<?nvs echo $value; ?>`. It writes one expression at the place where the
tag is. Inside `<?= ?>`, a `;` before `?>` is allowed and not needed. Inside `<?nvs ?>`, each statement
needs its `;`, also the last one before `?>`.

A file whose first line starts with `#!` is in code mode from its second line. The first line is not
written to the output. A `<?nvs` tag before the first `?>` in such a file does not compile (`E0009`).

In an HTTP request, the output escapes every `string` you write, so `<?= $title ?>` cannot add a tag to
the page. A `Core\Html\Markup` value is written as it is. You build one with an html template, and
`<?= $value ?>` inside that template escapes the value in the same way.
