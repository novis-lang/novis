// Shows a table of 100 products, with the search term in the heading.
// This measures building an HTML page and escaping text into it.
package main

import (
	"fmt"
	"html/template"
	"net/http"
)

type product struct {
	ID    int
	Name  string
	Price string
}

// `html/template` escapes every value by where it lands in the page; here all of them land in text.
var page = template.Must(template.New("page").Parse(`<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Products</title></head><body>
<h1>Results for {{.Term}}</h1>
<table>
<tr><th>Id</th><th>Name</th><th>Price</th></tr>
{{range .Products}}<tr><td>{{.ID}}</td><td>{{.Name}}</td><td>{{.Price}}</td></tr>
{{end}}</table>
</body></html>
`))

func main() {
	http.HandleFunc("GET /products", func(w http.ResponseWriter, r *http.Request) {
		term := r.URL.Query().Get("q")

		// The products are built again for every request, as a page reads them from a database.
		products := make([]product, 0, 100)
		for i := 1; i <= 100; i++ {
			cents := 100 + (i*7919)%90000
			products = append(products, product{ID: i, Name: fmt.Sprintf("Teapot %d & lid", i), Price: fmt.Sprintf("%d.%02d", cents/100, cents%100)})
		}

		w.Header().Set("Content-Type", "text/html; charset=UTF-8")
		page.Execute(w, struct {
			Term     string
			Products []product
		}{term, products})
	})
	http.ListenAndServe(":8080", nil)
}
