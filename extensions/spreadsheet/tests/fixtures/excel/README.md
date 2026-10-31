# Files made by Excel

Each `.xlsx` here was created in Excel. They are copied byte for byte from the test suite of
[rust_xlsxwriter](https://github.com/jmcnamara/rust_xlsxwriter) by John McNamara, from its
`tests/input/` folder. rust_xlsxwriter is dual-licensed under MIT or Apache-2.0, and these copies
are used under MIT. Its licence text is [LICENSE-MIT](LICENSE-MIT), next to this file.

`tests/excel.rs` writes the same workbook with this crate and compares the parts named below with
the file Excel made. A difference is fixed in the writer. A file here is never changed.

Each file belongs to the upstream test of the same name, `tests/integration/<name>.rs`, which shows
the workbook it holds.

| File | Upstream test | Compared |
|---|---|---|
| `format02.xlsx` | `format02` | `xl/styles.xml` |
| `format03.xlsx` | `format03` | `xl/styles.xml` |
| `format05.xlsx` | `format05` | `xl/styles.xml` |
| `format11.xlsx` | `format11` | `xl/styles.xml` |
| `format12.xlsx` | `format12` | `xl/styles.xml` |
| `format15.xlsx` | `format15` | `xl/styles.xml` |
| `format19.xlsx` | `format19` | `xl/styles.xml` |
| `table01.xlsx` | `table01` | `xl/tables/table1.xml` |
| `table10.xlsx` | `table10` | the calculated column's formula in each data cell |
| `table08.xlsx` | `table08` | `xl/tables/table1.xml` |
| `table23.xlsx` | `table23` | `xl/tables/table1.xml` |
| `table24.xlsx` | `table24` | `xl/tables/table1.xml` |
| `table28.xlsx` | `table28` | `xl/tables/table1.xml` |
| `table32.xlsx` | `table32` | `xl/tables/table1.xml` |
| `cond_format01.xlsx` | `cond_format01` | the sheet's conditional formats, `xl/styles.xml` |
| `cond_format05.xlsx` | `cond_format05` | the sheet's conditional formats, `xl/styles.xml` |
| `cond_format08.xlsx` | `cond_format08` | the sheet's conditional formats, `xl/styles.xml` |
| `cond_format10.xlsx` | `cond_format10` | the sheet's conditional formats, `xl/styles.xml` |
| `cond_format19.xlsx` | `cond_format19` | the sheet's conditional formats, `xl/styles.xml` |
| `data_validation01.xlsx` | `data_validation01` | the sheet's data validation |
| `data_validation02.xlsx` | `data_validation02` | the sheet's data validation |
| `data_validation03.xlsx` | `data_validation03` | the sheet's data validation |
| `image53.xlsx` | `image53` | `xl/drawings/drawing1.xml`, its relationships, the image |
| `chart_column01.xlsx` | `chart_column01` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
| `chart_bar01.xlsx` | `chart_bar01` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
| `chart_line01.xlsx` | `chart_line01` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
| `chart_area01.xlsx` | `chart_area01` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
| `chart_pie01.xlsx` | `chart_pie01` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
| `chart_doughnut01.xlsx` | `chart_doughnut01` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
| `chart_scatter01.xlsx` | `chart_scatter01` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
| `chart_combined02.xlsx` | `chart_combined02` | `xl/charts/chart1.xml`, `xl/drawings/drawing1.xml` |
