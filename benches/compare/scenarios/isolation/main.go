// Writes the value a package variable has when the request starts, then the value from `?v=`.
// A request that sees `0` first did not see the value an earlier request wrote.
package main

import (
	"fmt"
	"net/http"
	"strconv"
	"sync"
)

var (
	mu    sync.Mutex
	value int
)

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		seen := value
		value, _ = strconv.Atoi(r.URL.Query().Get("v"))
		now := value
		mu.Unlock()
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		fmt.Fprintf(w, "%d %d", seen, now)
	})
	http.ListenAndServe(":8080", nil)
}
