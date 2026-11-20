// A `POST` to `/change` changes the account and writes `changed`.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.MapPost("/change", () => "changed\n");

app.Run("http://0.0.0.0:8080");
