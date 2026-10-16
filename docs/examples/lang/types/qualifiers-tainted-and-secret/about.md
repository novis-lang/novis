Two words you can write in front of `string` and `bytes` to say where a value came from: `tainted`
for input you did not write yourself, `secret` for something confidential.

The compiler then follows those marks for you. A tainted value keeps its mark through joining and
interpolation, so it is still refused wherever it would be read as an instruction: a regular
expression, a format template, a date pattern. A secret value is refused by `echo`, by an error
message, by a debug dump and by `Core\Json::encode`. Neither mark costs anything while your program
runs. Both are checked before it starts, so a mistake is a message from the compiler rather than a
leak in production.

**In plain words:** a sticky label on a value. The label travels with the value everywhere it goes,
and certain doors will not let a labelled value through.

**Good to know:** a checked conversion such as `as int` takes the label off, because the answer is a
number rather than the text you were handed. `Core\Secret::reveal` is the one named way out of
`secret`, and it asks you to write down why.
