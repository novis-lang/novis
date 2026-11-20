# .NET's side of the comparison: every case under `cases/` and every row under `scenarios/` built once,
# one framework-dependent Release build per folder at `/srv/<tree>/<folder>/app.dll`.
#
# The build context is `benches/compare/`. A folder with no `Program.cs` is skipped, so a case or a row
# is added by writing its programs and nothing here changes. `App.csproj` is copied into each folder
# that has one. `appsettings.json` is the one `dotnet new web` writes, so ASP.NET Core logs warnings
# and not one line per request.

FROM mcr.microsoft.com/dotnet/sdk:10.0.404-alpine3.24 AS build

WORKDIR /src
COPY dotnet/App.csproj dotnet/appsettings.json ./dotnet/
COPY cases/ ./cases/
COPY scenarios/ ./scenarios/

RUN for tree in cases scenarios; do \
      for dir in "$tree"/*/; do \
        name="$(basename "$dir")"; \
        if [ -f "$tree/$name/Program.cs" ]; then \
          cp dotnet/App.csproj "$tree/$name/app.csproj" && \
          dotnet publish "$tree/$name/app.csproj" -c Release --nologo -o "/out/$tree/$name" && \
          cp dotnet/appsettings.json "/out/$tree/$name/" || exit 1; \
        fi; \
      done; \
    done

# The Alpine runtime image, because the driver reads the cgroup's memory files with `cat` and the
# healthcheck needs `nc`.
FROM mcr.microsoft.com/dotnet/aspnet:10.0.15-alpine3.24

COPY --from=build /out/ /srv/
EXPOSE 8080
