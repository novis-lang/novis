// Waits 50 ms on a timer, then answers. This measures the cost of many
// requests in flight at once, as when a page waits for another service.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.MapGet("/wait", async () =>
{
    await Task.Delay(50);
    return Results.Text("waited 50 ms\n", "text/plain; charset=UTF-8");
});

app.Run("http://0.0.0.0:8080");
