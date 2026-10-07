Chooses the language for a page from the `Accept-Language` header a browser sends.

`Locale::negotiate` takes the header's value, the list of locales your site offers, and a default. It
returns the offered locale that the reader wants most. The header lists languages with weights, such
as `"fr;q=0.5, de, en;q=0.8"`. A language with no weight has the weight 1. For that header and the
offered list `["en", "de", "fr"]`, the result is `"de"`.

A regional tag matches its language. `"de-AT"` and `"de-CH"` both match an offered `"de"`. The result
is always written the way your list writes it, so `"pt-br"` in the header gives `"pt-BR"` when you
offer `"pt-BR"`.

An empty header, a header that is not valid, and a header that matches nothing all return the default.
The method does not throw for them, because the header comes from the reader and may contain
anything. It throws a `LogicError` when a tag in your own offered list is not valid.

**Good to know:** the result is always one of your offered locales or the default. You can pass it
to the other `Novis\Intl` classes without checking it again.
