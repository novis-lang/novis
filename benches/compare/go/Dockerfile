# Go's side of the comparison: every case under `cases/` compiled once, one binary per folder.
#
# The build context is `benches/compare/`. A case folder with no `main.go` is skipped, so a case is
# added by writing its four programs and nothing here changes.

FROM golang:1.27.2-alpine AS build

WORKDIR /src
COPY cases/ ./

# Static binaries, so the runtime image needs no libc of Go's choosing.
RUN mkdir /out \
 && for dir in */; do \
      name="${dir%/}"; \
      if [ -f "$name/main.go" ]; then CGO_ENABLED=0 go build -trimpath -o "/out/$name" "./$name" || exit 1; fi; \
    done

# Alpine rather than `scratch`, because the driver reads the cgroup's memory files with `cat` and the
# healthcheck needs `nc`.
FROM alpine:3.23.0

COPY --from=build /out/ /srv/
EXPOSE 8080
