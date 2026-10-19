# The `nvs` origin. `bun nv garden` stages this file as `Dockerfile` beside a Linux `nvs` and
# `echo.nvs`, so the build context holds those three files and nothing else.
FROM debian:trixie-slim

COPY nvs /usr/local/bin/nvs
RUN chmod +x /usr/local/bin/nvs
COPY echo.nvs /app/echo.nvs

WORKDIR /app
EXPOSE 80
ENTRYPOINT ["nvs"]
CMD ["serve", "/app/echo.nvs", "--listen", "0.0.0.0:80"]
