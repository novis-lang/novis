// Shows a table of 100 products, with the search term in the heading.
// This measures building an HTML page and escaping text into it.

interface Product {
  id: number;
  name: string;
  priceCents: number;
}

const html = { "Content-Type": "text/html; charset=UTF-8" };

export default {
  fetch(req: Request): Response {
    const url = new URL(req.url);
    if (url.pathname !== "/products") {
      return new Response("not found\n", { status: 404, headers: { "Content-Type": "text/plain; charset=UTF-8" } });
    }
    const term = url.searchParams.get("q") ?? "";

    // The products are built again for every request, as a page reads them from a database.
    const products: Product[] = [];
    for (let i = 1; i <= 100; i++) {
      products.push({ id: i, name: `Teapot ${i} & lid`, priceCents: 100 + ((i * 7919) % 90000) });
    }

    // `Bun.escapeHTML` escapes `&`, `<`, `>`, `"` and `'` in the term and in every name.
    let rows = "";
    for (const p of products) {
      const price = `${Math.floor(p.priceCents / 100)}.${String(p.priceCents % 100).padStart(2, "0")}`;
      rows += `<tr><td>${p.id}</td><td>${Bun.escapeHTML(p.name)}</td><td>${Bun.escapeHTML(price)}</td></tr>\n`;
    }

    const page =
      "<!DOCTYPE html>\n" +
      '<html><head><meta charset="utf-8"><title>Products</title></head><body>\n' +
      `<h1>Results for ${Bun.escapeHTML(term)}</h1>\n` +
      "<table>\n" +
      "<tr><th>Id</th><th>Name</th><th>Price</th></tr>\n" +
      rows +
      "</table>\n" +
      "</body></html>\n";
    return new Response(page, { headers: html });
  },
};
