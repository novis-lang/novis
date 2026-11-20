// Runs `?parts=` parts of the same loop and writes the sum of their results.
// Each part runs in its own `Task.Run`, and .NET spreads the tasks over the cores.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.Map("/{**path}", async (string? parts, string? turns) =>
{
    int.TryParse(parts, out var count);
    int.TryParse(turns, out var n);
    var tasks = new Task<long>[count];
    for (var k = 0; k < count; k++)
    {
        var seed = k + 1;
        tasks[k] = Task.Run(() => Part(seed, n));
    }
    var results = await Task.WhenAll(tasks);
    long sum = 0;
    foreach (var x in results)
    {
        sum += x;
    }
    return Results.Text(sum.ToString(), "text/html; charset=UTF-8");
});

app.Run("http://0.0.0.0:8080");

static long Part(long seed, int turns)
{
    var x = seed;
    for (var i = 0; i < turns; i++)
    {
        x = (x * 31 + i) % 1000003;
    }
    return x;
}
