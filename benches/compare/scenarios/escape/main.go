// Writes the search term `?q=` into a heading with `html/template` on `/`, and with
// `fmt.Fprintf` on `/direct`.
package main

import (
	"fmt"
	"html/template"
	"net/http"
)

var page = template.Must(template.New("page").Parse("<h1>Results for {{.}}</h1>\n"))

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		page.Execute(w, r.URL.Query().Get("q"))
	})
	http.HandleFunc("/direct", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		fmt.Fprintf(w, "<h1>Results for %s</h1>\n", r.URL.Query().Get("q"))
	})
	http.ListenAndServe(":8080", nil)
}
