// Loops forever for `/loop`, and adds 1 MiB to a slice forever for `/alloc`.
// Every other path writes `ok`.
package main

import (
	"io"
	"net/http"
	"strings"
)

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/loop" {
			for {
			}
		}
		if r.URL.Path == "/alloc" {
			var chunks []string
			for {
				chunks = append(chunks, strings.Repeat("x", 1048576))
			}
		}
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		io.WriteString(w, "ok\n")
	})
	http.ListenAndServe(":8080", nil)
}
