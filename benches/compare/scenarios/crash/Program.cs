// Throws an exception for the path `/crash`, and writes `ok` for every other path.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

// Nothing catches the exception, so ASP.NET Core answers with status 500.
app.Map("/crash", () =>
{
    throw new InvalidOperationException("the handler failed");
});

app.Map("/{**path}", () => Results.Text("ok\n", "text/html; charset=UTF-8"));

app.Run("http://0.0.0.0:8080");
