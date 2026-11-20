// Loops forever for `/loop`, and adds 1 MiB to a list forever for `/alloc`.
// Every other path writes `ok`.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.Map("/loop", () =>
{
    while (true)
    {
    }
});

app.Map("/alloc", () =>
{
    // A .NET `char` is 2 bytes, so a string of 512 Ki characters is 1 MiB.
    var chunks = new List<string>();
    while (true)
    {
        chunks.Add(new string('x', 524288));
    }
});

app.Map("/{**path}", () => Results.Text("ok\n", "text/html; charset=UTF-8"));

app.Run("http://0.0.0.0:8080");
