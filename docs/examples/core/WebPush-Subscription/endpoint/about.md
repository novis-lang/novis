Returns the URL of the browser's push service for this subscription, for example
`https://push.example.com/send/abc123`. Each browser has its own URL, so you can use it to find a
saved subscription again. For example, you delete a subscription by its endpoint when
`Core\WebPush\Sent::gone()` is true.

The browser wrote this URL, so `endpoint()` returns a `tainted string`. Tainted means that the text
came from outside your program and nothing has checked it yet. You can print it and compare it.
To put it into an HTML page, use `Core\Html::escape`. The HTTP client does not accept a tainted
URL, so your program does not compile if it fetches the endpoint itself. `Core\WebPush::send` is
the way to send to it.

**Good to know:** `Core\WebPush::subscription` throws an error for an endpoint that is not an
`https:` URL, so `endpoint()` always returns an `https:` URL.

The examples read the endpoint, delete the subscriptions that are gone, and show the endpoint in
an HTML page.
