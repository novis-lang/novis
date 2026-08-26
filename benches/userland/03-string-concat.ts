// Building output one piece at a time - the shape of every hand-rolled template and CSV writer.
// `+=` is the JavaScript idiom here: the engine builds a rope rather than copying, which is why a
// JS author does not reach for the array-and-join Python needs.

function run(rows: number): string {
    let out = "";
    for (let i = 0; i < rows; i++) {
        out += "<tr><td>" + i + "</td><td>row-" + i + "</td></tr>";
    }
    return out;
}

const html = run(20000);
console.log(html.length);
