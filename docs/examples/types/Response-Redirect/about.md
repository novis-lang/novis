Which kind of redirect a response is, when it answers by sending the visitor somewhere else.

There are three. `SeeOther` sends the browser to fetch another page with a plain `GET`, whatever
method asked — the answer to a saved form, and what you get when you name no case at all.
`Temporary` asks for the same request again, method and body intact, at the new address this once.
`Permanent` asks for the same request again as well, and says the new address is the one from now
on, so caches and search engines are entitled to remember it.

**Good to know:** the two redirects everybody knows, `301` and `302`, are missing on purpose. What a
browser does with a posted form under those was settled by habit rather than by the standard, so what
happens depends on who is asking. The three here mean the same thing everywhere, and any other status
is still yours to write by hand.
