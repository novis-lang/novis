// A hot integer loop: the arithmetic every counter, accumulator and offset walk is made of.
// JavaScript's `%` truncates toward zero exactly as PHP's and Novis's do, so this case needs no
// helper the way its Python twin does -- see the README's note on arithmetic that has to agree.

function run(rounds: number): number {
    let total = 0;
    for (let i = 0; i < rounds; i++) {
        total = (total + i * 3 - 1) % 1000003;
    }
    return total;
}

console.log(run(3000000));
