Builds a link to one of your routes that nobody can change. `Core\Router::urlSigned` builds the same
link as `Core\Router::url` and adds a signature in the `_sig` query parameter. You give it the
method that has the `#[Route]`, such as `Reports::download`, the values, your keys, and the time
when the link stops working. The values are always required, even an empty array.

The signature covers the route's path, such as `/unsubscribe/{reader}`, every value and the end
time. If somebody changes a value in the link, the signature does not match any more.
`Core\Router::signedRoute` checks the signature when the request arrives.

The method's name is not in the signature. If you rename the method, links that you sent before
still work. If you change the path in its `#[Route]`, those links stop working.

Use a signed link when the link itself is the permission: a download that works for one hour, an
unsubscribe link, or an invitation. Keep the keys secret. Anybody who has a key can make links.

The examples show a link that works for one hour, how the signature follows every value, and
unsubscribe links for a newsletter.
