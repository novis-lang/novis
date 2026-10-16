Names the one audience Novis itself understands: everyone.

Every route in a Novis program says who may reach it, and it says so by naming a value rather than by
writing a piece of text. `Core\Audience::Public` is the name for *anyone may call this route*, and it
is the only case this enum has. It will never gain a second one. Every other answer — a signed-in
customer, an administrator, somebody on one team — belongs to your application, so you write your own
enum or class constant for it and give that to the route instead. Novis checks that the name you
wrote exists. It never decides what the name means.

**Good to know:** a plain string or number is refused while your program is compiled, so a misspelt
`"public"` can never quietly open a route.
