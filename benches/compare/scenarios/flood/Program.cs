// `/hold` keeps 8 MiB for 300 ms and then writes `held`.
// Every other path writes `ok`.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.Map("/hold", async () =>
{
    // A .NET `char` is 2 bytes, so a string of 4 Mi characters is 8 MiB.
    var held = new string('x', 4194304);
    await Task.Delay(300);
    return held.EndsWith('x') ? Results.Text("held\n", "text/html; charset=UTF-8") : Results.Empty;
});

app.Map("/{**path}", () => Results.Text("ok\n", "text/html; charset=UTF-8"));

app.Run("http://0.0.0.0:8080");
