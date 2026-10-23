Sends the browser to another address. It sets a redirect status and the `Location` header with the
new address. The browser then loads the new address by itself.

`Core\Response::Redirect` names the three kinds of redirect. `SeeOther` (303) is the default. Use it
after a form is sent, so the browser loads the next page with `GET`. `Temporary` (307) and
`Permanent` (308) repeat the same request at the new address. `Permanent` also tells the browser to
use the new address from now on.

The address can be a full URL or a path such as `/orders/42`. It must not be empty, and it contains
only printable ASCII characters. It must not start with two slashes or backslashes, such as
`//example.com`, because a browser reads that as another host. For another host, write the scheme,
such as `https://example.com/`. For any other address, `redirect` throws a `LogicError` and sets
nothing. Text from a visitor as the address does not compile, because a visitor must not choose where
the browser goes.

The examples show a page that moved, an address that is not allowed, and a form that saves an order.
