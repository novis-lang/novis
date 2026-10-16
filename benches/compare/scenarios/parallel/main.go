// Runs `?parts=` parts of the same loop and writes the sum of their results.
// Each part runs in its own goroutine, and Go spreads goroutines over the cores.
package main

import (
	"fmt"
	"net/http"
	"strconv"
	"sync"
)

func part(seed, turns int) int {
	x := seed
	for i := 0; i < turns; i++ {
		x = (x*31 + i) % 1000003
	}
	return x
}

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		parts, _ := strconv.Atoi(r.URL.Query().Get("parts"))
		turns, _ := strconv.Atoi(r.URL.Query().Get("turns"))
		results := make([]int, parts)
		var wg sync.WaitGroup
		for k := range parts {
			wg.Add(1)
			go func() {
				defer wg.Done()
				results[k] = part(k+1, turns)
			}()
		}
		wg.Wait()
		sum := 0
		for _, x := range results {
			sum += x
		}
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		fmt.Fprintf(w, "%d", sum)
	})
	http.ListenAndServe(":8080", nil)
}
