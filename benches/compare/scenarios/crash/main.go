// Panics for the path `/crash`, and writes `ok` for every other path.
package main

import (
	"io"
	"net/http"
)

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/crash" {
			panic("the handler failed")
		}
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		io.WriteString(w, "ok\n")
	})
	http.ListenAndServe(":8080", nil)
}
