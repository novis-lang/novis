// Writes the version of each part. Go compiles the parts into one binary,
// so a new version needs a new image and a restart. `DEPLOY_PARTS` is the
// number of parts, 200 unless the row sets it.
package main

import (
	"io"
	"net/http"
	"os"
	"strconv"
	"strings"
)

func main() {
	parts, err := strconv.Atoi(os.Getenv("DEPLOY_PARTS"))
	if err != nil {
		parts = 200
	}
	body := strings.Repeat("1", parts) + "\n"
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		io.WriteString(w, body)
	})
	http.ListenAndServe(":8080", nil)
}
