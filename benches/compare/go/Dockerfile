# Go's side of the comparison: every case under `cases/` and every row under `scenarios/` compiled
# once, one binary per folder at `/srv/<tree>/<folder>`.
#
# The build context is `benches/compare/`, and each tree is its own module. A folder with no `main.go`
# is skipped, so a case or a row is added by writing its four programs and nothing here changes.

FROM golang:1.27.3-alpine AS build

WORKDIR /src
COPY cases/ ./cases/
COPY scenarios/ ./scenarios/

# Static binaries, so the runtime image needs no libc of Go's choosing.
RUN for tree in cases scenarios; do \
      mkdir -p "/out/$tree"; \
      for dir in "$tree"/*/; do \
        name="$(basename "$dir")"; \
        if [ -f "$tree/$name/main.go" ]; then \
          (cd "$tree" && CGO_ENABLED=0 go build -trimpath -o "/out/$tree/$name" "./$name") || exit 1; \
        fi; \
      done; \
    done

# Alpine rather than `scratch`, because the driver reads the cgroup's memory files with `cat` and the
# healthcheck needs `nc`.
FROM alpine:3.24.2

COPY --from=build /out/ /srv/
EXPOSE 8080
