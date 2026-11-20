// Writes the search term `?q=` into a heading with `HtmlEncoder` on `/`, and with
// no escaping on `/direct`.
using System.Text.Encodings.Web;
using System.Text.Unicode;

var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

// `HtmlEncoder` escapes `<`, `>`, `&`, `'` and `"`. With all ranges allowed, it does not escape letters such as `é`.
var html = HtmlEncoder.Create(UnicodeRanges.All);

app.Map("/direct", (string? q) => Results.Text($"<h1>Results for {q}</h1>\n", "text/html; charset=UTF-8"));

app.Map("/{**path}", (string? q) => Results.Text($"<h1>Results for {html.Encode(q ?? "")}</h1>\n", "text/html; charset=UTF-8"));

app.Run("http://0.0.0.0:8080");
