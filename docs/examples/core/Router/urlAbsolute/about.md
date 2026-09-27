Builds the full URL of one of your routes. `Core\Router::urlAbsolute` works like `Core\Router::url`:
you give it the name of a `#[Route]` and an array of values. The result has the origin of your site
in front, for example `https://www.example.com/articles/42`.

You need a full URL wherever the link is read outside your site: in an email, a sitemap, or a
message to another service.

The origin is the `origin` setting of the `[[app]]` block in `nvs.toml`. It never comes from the
`Host` header of a request, so a visitor cannot change the site that your links point to. If no
origin is set, the call throws a `RuntimeError`.

The examples show a link with and without the origin, a sitemap, and a link in an email.
