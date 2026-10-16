// A substitution over a string - the slug, the sanitiser, the whitespace collapse.

const NON_ALNUM = /[^A-Za-z0-9]+/g;

function run(rounds: number): number {
    const raws: string[] = [];
    for (let k = 0; k < 16; k++) {
        raws.push("  The Quick,  Brown FOX -- jumped over " + "2 ".repeat(k + 1) + "lazy dogs!!  ");
    }

    let total = 1;
    for (let i = 0; i < rounds; i++) {
        const slug = raws[total % 16].replace(NON_ALNUM, "-");
        total = total + slug.length;
    }
    return total;
}

console.log(run(200000));
