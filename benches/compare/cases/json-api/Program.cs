// Reads a JSON order, checks it, and returns its totals in cents as JSON.
// This measures reading, checking and writing JSON in a typical API request.
using System.Text.Encodings.Web;
using System.Text.Json;

var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

// Names are matched without case when reading, and written in camelCase.
// `UnsafeRelaxedJsonEscaping` writes `<`, `>`, `&` and letters such as `é` as they are, as the other peers write them.
var json = new JsonSerializerOptions
{
    PropertyNameCaseInsensitive = true,
    PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
};

app.MapPost("/orders", async (HttpContext ctx) =>
{
    Receipt? receipt = null;
    try
    {
        var order = await JsonSerializer.DeserializeAsync<Order>(ctx.Request.Body, json);
        receipt = Total(order);
    }
    catch (JsonException)
    {
    }
    if (receipt is null)
    {
        await WriteJson(ctx, StatusCodes.Status422UnprocessableEntity, new { error = "invalid order" });
        return;
    }
    await WriteJson(ctx, StatusCodes.Status200OK, receipt);
});

app.Run("http://0.0.0.0:8080");

// Returns the receipt for one decoded order, or null when the order fails a check.
static Receipt? Total(Order? o)
{
    var c = o?.Customer;
    if (o is null || c is null || c.Id is null || c.Name is null || c.Email is null || o.Items is null)
    {
        return null;
    }
    if (c.Id < 1 || o.Items.Count < 1 || o.Items.Count > 100)
    {
        return null;
    }
    var lines = new List<Line>(o.Items.Count);
    long subtotal = 0;
    foreach (var it in o.Items)
    {
        if (it is null || it.Sku is null || it.Name is null || it.Qty is not long qty || it.UnitCents is not long unitCents)
        {
            return null;
        }
        if (it.Sku == "" || qty < 1 || qty > 1000 || unitCents < 1 || unitCents > 10_000_000)
        {
            return null;
        }
        var t = qty * unitCents;
        lines.Add(new Line(it.Sku, qty, t));
        subtotal += t;
    }

    // The tax is 20 percent, rounded to the nearest cent. Half a cent rounds up.
    var tax = (subtotal * 20 + 50) / 100;

    // The largest line comes first. `OrderByDescending` is stable, so lines with the same total keep the order of the request.
    var sorted = lines.OrderByDescending(l => l.TotalCents).ToList();

    return new Receipt(c.Id.Value, sorted, subtotal, tax, subtotal + tax);
}

// Writes compact JSON with the type `application/json` and no charset, as the other peers send it.
async Task WriteJson(HttpContext ctx, int status, object value)
{
    ctx.Response.StatusCode = status;
    ctx.Response.ContentType = "application/json";
    await ctx.Response.WriteAsync(JsonSerializer.Serialize(value, json));
}

// Every property can be null, so a missing field is null and fails the check.
record Customer(long? Id, string? Name, string? Email);

record Item(string? Sku, string? Name, long? Qty, long? UnitCents);

record Order(Customer? Customer, List<Item?>? Items);

record Line(string Sku, long Qty, long TotalCents);

record Receipt(long Customer, List<Line> Items, long SubtotalCents, long TaxCents, long TotalCents);
