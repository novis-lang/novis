// Waits 50 ms on a timer, then answers. This measures the cost of many
// requests in flight at once, as when a page waits for another service.
package main

import (
	"net/http"
	"time"
)

func main() {
	http.HandleFunc("GET /wait", func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(50 * time.Millisecond)
		w.Header().Set("Content-Type", "text/plain; charset=UTF-8")
		w.Write([]byte("waited 50 ms\n"))
	})
	http.ListenAndServe(":8080", nil)
}
