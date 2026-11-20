// Shows a table of 100 products, with the search term in the heading.
// This measures building an HTML page and escaping text into it.

// Node has no HTML escaper in its standard library, so this function escapes text for HTML.
// It replaces `&`, `<`, `>`, `"` and `'` with their HTML entities.
function escapeHTML(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

export default function (req, res) {
  const url = new URL(req.url, "http://localhost");
  if (url.pathname !== "/products") {
    res.writeHead(404, { "Content-Type": "text/plain; charset=UTF-8" });
    res.end("not found\n");
    return;
  }
  const term = url.searchParams.get("q") ?? "";

  // The products are built again for every request, as a page reads them from a database.
  const products = [];
  for (let i = 1; i <= 100; i++) {
    products.push({ id: i, name: `Teapot ${i} & lid`, priceCents: 100 + ((i * 7919) % 90000) });
  }

  // `escapeHTML` escapes the term and every name.
  let rows = "";
  for (const p of products) {
    const price = `${Math.floor(p.priceCents / 100)}.${String(p.priceCents % 100).padStart(2, "0")}`;
    rows += `<tr><td>${p.id}</td><td>${escapeHTML(p.name)}</td><td>${escapeHTML(price)}</td></tr>\n`;
  }

  const page =
    "<!DOCTYPE html>\n" +
    '<html><head><meta charset="utf-8"><title>Products</title></head><body>\n' +
    `<h1>Results for ${escapeHTML(term)}</h1>\n` +
    "<table>\n" +
    "<tr><th>Id</th><th>Name</th><th>Price</th></tr>\n" +
    rows +
    "</table>\n" +
    "</body></html>\n";
  res.writeHead(200, { "Content-Type": "text/html; charset=UTF-8" });
  res.end(page);
}
