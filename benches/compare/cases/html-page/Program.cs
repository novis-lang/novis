// Shows a table of 100 products, with the search term in the heading.
// This measures building an HTML page and escaping text into it.
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Unicode;

var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

// `HtmlEncoder` escapes `<`, `>`, `&`, `'` and `"`. With all ranges allowed, it does not escape letters such as `é`.
var html = HtmlEncoder.Create(UnicodeRanges.All);

app.MapGet("/products", (string? q) =>
{
    // The products are built again for every request, as a page reads them from a database.
    var products = new List<Product>(100);
    for (var i = 1; i <= 100; i++)
    {
        var cents = 100 + (i * 7919) % 90000;
        products.Add(new Product(i, $"Teapot {i} & lid", $"{cents / 100}.{cents % 100:D2}"));
    }

    var page = new StringBuilder();
    page.Append("<!DOCTYPE html>\n");
    page.Append("<html><head><meta charset=\"utf-8\"><title>Products</title></head><body>\n");
    page.Append("<h1>Results for ");
    page.Append(html.Encode(q ?? ""));
    page.Append("</h1>\n<table>\n<tr><th>Id</th><th>Name</th><th>Price</th></tr>\n");
    foreach (var p in products)
    {
        page.Append($"<tr><td>{p.Id}</td><td>{html.Encode(p.Name)}</td><td>{html.Encode(p.Price)}</td></tr>\n");
    }
    page.Append("</table>\n</body></html>\n");

    return Results.Text(page.ToString(), "text/html; charset=UTF-8");
});

app.Run("http://0.0.0.0:8080");

record Product(int Id, string Name, string Price);
