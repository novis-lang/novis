Sends plain text as the response. It also adds the header `Content-Type: text/plain;
charset=utf-8`, so the browser shows the body as text and not as a page.

The body is a `string`, and it is sent exactly as it is. Nothing is escaped or changed. Text from
a visitor is allowed. The server also sends the header `X-Content-Type-Options: nosniff`, so a
browser never reads the text as HTML. A `<script>` tag in the text is shown as text and does not
run.

Use `Core\Response::sendHtml` for a page, `sendJson` for data that a program reads, and `sendBytes`
for any other content type. Using `sendText` and `echo` in one response does not compile.

The examples show a health check, text from a visitor, and a `robots.txt` file built from a list.
