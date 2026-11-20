// Writes one line for every request. This measures the server alone.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.Map("/{**path}", () => Results.Text("hello, world\n", "text/html; charset=UTF-8"));

app.Run("http://0.0.0.0:8080");
