Which cross-site requests carry your cookie. A cookie belongs to your site, and this setting decides
whether it also travels on a request that somebody else's page made to yours.

`Lax` sends it when a visitor clicks through to your site, and with nothing else — not with an image,
a form post or a script call from elsewhere. That is what a cookie you say nothing about gets.
`Strict` never sends it across a site boundary at all, so somebody arriving from a link arrives
signed out. `None` sends it from anywhere, and a cookie that travels that far has to be marked secure
as well; the pair without it is refused, because a browser drops such a cookie rather than storing
it.

**Good to know:** this is the setting a cross-site request forgery has to get past. A form on
somebody else's page can still post to yours; what `Lax` stops is that post arriving with your
visitor's session.
