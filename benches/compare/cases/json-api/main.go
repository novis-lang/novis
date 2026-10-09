// Reads a JSON order, checks it, and returns its totals in cents as JSON.
// This measures reading, checking and writing JSON in a typical API request.
package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"sort"
)

// Pointers, so a missing field is nil and fails the check.
type customer struct {
	ID    *int    `json:"id"`
	Name  *string `json:"name"`
	Email *string `json:"email"`
}

type item struct {
	SKU       *string `json:"sku"`
	Name      *string `json:"name"`
	Qty       *int    `json:"qty"`
	UnitCents *int    `json:"unitCents"`
}

type order struct {
	Customer *customer `json:"customer"`
	Items    []item    `json:"items"`
}

type line struct {
	SKU        string `json:"sku"`
	Qty        int    `json:"qty"`
	TotalCents int    `json:"totalCents"`
}

type receipt struct {
	Customer      int    `json:"customer"`
	Items         []line `json:"items"`
	SubtotalCents int    `json:"subtotalCents"`
	TaxCents      int    `json:"taxCents"`
	TotalCents    int    `json:"totalCents"`
}

// Returns the receipt for one decoded order, or nil when the order fails a check.
func total(o *order) *receipt {
	c := o.Customer
	if c == nil || c.ID == nil || c.Name == nil || c.Email == nil || o.Items == nil {
		return nil
	}
	if *c.ID < 1 || len(o.Items) < 1 || len(o.Items) > 100 {
		return nil
	}
	lines := make([]line, 0, len(o.Items))
	subtotal := 0
	for _, it := range o.Items {
		if it.SKU == nil || it.Name == nil || it.Qty == nil || it.UnitCents == nil {
			return nil
		}
		if *it.SKU == "" || *it.Qty < 1 || *it.Qty > 1000 || *it.UnitCents < 1 || *it.UnitCents > 10000000 {
			return nil
		}
		t := *it.Qty * *it.UnitCents
		lines = append(lines, line{SKU: *it.SKU, Qty: *it.Qty, TotalCents: t})
		subtotal += t
	}

	// The tax is 20 percent, rounded to the nearest cent. Half a cent rounds up.
	tax := (subtotal*20 + 50) / 100

	// The largest line comes first. Lines with the same total keep the order of the request.
	sort.SliceStable(lines, func(a, b int) bool { return lines[a].TotalCents > lines[b].TotalCents })

	return &receipt{Customer: *c.ID, Items: lines, SubtotalCents: subtotal, TaxCents: tax, TotalCents: subtotal + tax}
}

// Writes compact JSON without escaping `<`, `>` and `&`, as the other peers write it.
func writeJSON(w http.ResponseWriter, status int, value any) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	enc.Encode(value)
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	w.Write(bytes.TrimSuffix(buf.Bytes(), []byte("\n")))
}

func main() {
	http.HandleFunc("POST /orders", func(w http.ResponseWriter, r *http.Request) {
		body, err := io.ReadAll(r.Body)
		var o order
		var rc *receipt
		if err == nil && json.Unmarshal(body, &o) == nil {
			rc = total(&o)
		}
		if rc == nil {
			writeJSON(w, http.StatusUnprocessableEntity, map[string]string{"error": "invalid order"})
			return
		}
		writeJSON(w, http.StatusOK, rc)
	})
	http.ListenAndServe(":8080", nil)
}
