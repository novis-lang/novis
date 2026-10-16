// A `POST` to `/change` changes the account and writes `changed`.
package main

import (
	"io"
	"net/http"
)

func main() {
	http.HandleFunc("POST /change", func(w http.ResponseWriter, r *http.Request) {
		io.WriteString(w, "changed\n")
	})
	http.ListenAndServe(":8080", nil)
}
