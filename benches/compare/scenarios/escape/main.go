// Writes the search term `?q=` into a heading with `html/template`.
package main

import (
	"html/template"
	"net/http"
)

var page = template.Must(template.New("page").Parse("<h1>Results for {{.}}</h1>\n"))

func main() {
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		page.Execute(w, r.URL.Query().Get("q"))
	})
	http.ListenAndServe(":8080", nil)
}
