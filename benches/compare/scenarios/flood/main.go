// `/hold` keeps 8 MiB for 300 ms and then writes `held`.
// Every other path writes `ok`.
package main

import (
	"io"
	"net/http"
	"strings"
	"time"
)

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		if r.URL.Path == "/hold" {
			held := strings.Repeat("x", 8388608)
			time.Sleep(300 * time.Millisecond)
			if strings.HasSuffix(held, "x") {
				io.WriteString(w, "held\n")
			}
			return
		}
		io.WriteString(w, "ok\n")
	})
	http.ListenAndServe(":8080", nil)
}
