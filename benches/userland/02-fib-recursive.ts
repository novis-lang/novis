// Recursive calls: the function-call and return path, unrolled by naive fibonacci.

function fib(n: number): number {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

console.log(fib(30));
