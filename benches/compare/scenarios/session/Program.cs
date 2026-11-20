// One visitor's session, read and written by two kinds of request.
// `/slow` adds 1 to a counter in the session, then works for 2 seconds.
// `/fast` only reads the counter. Every other path starts the session.
var builder = WebApplication.CreateBuilder(args);
builder.Services.AddDistributedMemoryCache();
builder.Services.AddSession();

var app = builder.Build();
app.UseSession();

app.Map("/slow", async (HttpContext ctx) =>
{
    await ctx.Session.LoadAsync();
    ctx.Session.SetInt32("count", (ctx.Session.GetInt32("count") ?? 0) + 1);
    await Task.Delay(2000);
    return Results.Text("slow done\n", "text/plain; charset=UTF-8");
});

app.Map("/fast", async (HttpContext ctx) =>
{
    await ctx.Session.LoadAsync();
    return Results.Text($"count: {ctx.Session.GetInt32("count") ?? 0}\n", "text/plain; charset=UTF-8");
});

// ASP.NET Core sends the session cookie only once the session has a value, so this path stores the counter.
app.Map("/{**path}", async (HttpContext ctx) =>
{
    await ctx.Session.LoadAsync();
    ctx.Session.SetInt32("count", 0);
    return Results.Text("started\n", "text/plain; charset=UTF-8");
});

app.Run("http://0.0.0.0:8080");
