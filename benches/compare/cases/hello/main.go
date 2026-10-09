// Writes one line for every request. This measures the server alone.
package main

import (
	"io"
	"net/http"
)

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		io.WriteString(w, "hello, world\n")
	})
	http.ListenAndServe(":8080", nil)
}
