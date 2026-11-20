// Writes the version of each part. .NET compiles the parts into one program,
// so a new version needs a new image and a restart. `DEPLOY_PARTS` is the
// number of parts, 200 unless the row sets it.
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

if (!int.TryParse(Environment.GetEnvironmentVariable("DEPLOY_PARTS"), out var parts))
{
    parts = 200;
}
var body = new string('1', parts) + "\n";

app.Map("/{**path}", () => Results.Text(body, "text/html; charset=UTF-8"));

app.Run("http://0.0.0.0:8080");
