# Shows a table of 100 products, with the search term in the heading.
# This measures building an HTML page and escaping text into it.
from html import escape
from urllib.parse import parse_qs


async def respond(send, status, content_type, body):
    await send({"type": "http.response.start", "status": status, "headers": [(b"content-type", content_type)]})
    await send({"type": "http.response.body", "body": body.encode()})


async def app(scope, receive, send):
    if scope["path"] != "/products":
        await respond(send, 404, b"text/plain; charset=UTF-8", "not found\n")
        return
    term = parse_qs(scope["query_string"].decode("ascii")).get("q", [""])[0]

    # The products are built again for every request, as a page reads them from a database.
    products = [{"id": i, "name": f"Teapot {i} & lid", "price_cents": 100 + (i * 7919) % 90000} for i in range(1, 101)]

    # `html.escape` escapes the term and every name.
    rows = []
    for p in products:
        price = f"{p['price_cents'] // 100}.{p['price_cents'] % 100:02d}"
        rows.append(f"<tr><td>{p['id']}</td><td>{escape(p['name'])}</td><td>{escape(price)}</td></tr>\n")

    page = (
        "<!DOCTYPE html>\n"
        '<html><head><meta charset="utf-8"><title>Products</title></head><body>\n'
        f"<h1>Results for {escape(term)}</h1>\n"
        "<table>\n"
        "<tr><th>Id</th><th>Name</th><th>Price</th></tr>\n"
        + "".join(rows)
        + "</table>\n"
        "</body></html>\n"
    )
    await respond(send, 200, b"text/html; charset=UTF-8", page)
