Sends the browser to another address. It sets two things at once: a redirect status, and the
`Location` header with the new address. The browser then asks for the new address by itself.

There are three kinds of redirect, and `Core\Response\Redirect` names them. `SeeOther` (303) is the
default. You use it after a form is sent, so the browser loads the next page with `GET`. `Temporary`
(307) and `Permanent` (308) repeat the same request at the new address. `Permanent` also tells the
browser to use the new address from now on.

The address can be a full URL or a path such as `/orders/42`. It must not be empty and contains only
printable ASCII characters. For any other address, `redirect` throws a `LogicError` and sets nothing.
Text from a visitor is not allowed as the address, and does not compile, because a visitor must not
choose where the browser goes.

The examples show a page that moved, an address that is not allowed, and a form that saves an
order.
