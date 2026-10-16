Builds a link to one of your routes that nobody can change. `Core\Router::urlSigned` builds the same
link as `Core\Router::url` and adds a signature in the `_sig` query parameter. You give it the
route name, the values, your keys, and the time when the link stops working.

The signature covers the route name, every value and the end time. If somebody changes a value in
the link, the signature does not match any more. `Core\Router::signedRoute` checks the signature
when the request arrives.

Use a signed link when the link itself is the permission: a download that works for one hour, an
unsubscribe link, or an invitation. Keep the keys secret. Anybody who has a key can make links.

The examples show a link that works for one hour, how the signature follows every value, and
unsubscribe links for a newsletter.
