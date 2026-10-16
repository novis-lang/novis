// Writes the version of each of the 200 parts. Go compiles the parts into one binary,
// so a new version needs a new image and a restart.
package main

import (
	"io"
	"net/http"
	"strings"
)

func main() {
	body := strings.Repeat("1", 200) + "\n"
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		io.WriteString(w, body)
	})
	http.ListenAndServe(":8080", nil)
}
