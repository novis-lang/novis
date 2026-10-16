Checks whether a text is a MAC address. A MAC address is the hardware address of a network card.
The method returns `true` or `false`.

There are three ways to write a MAC address. The first is six pairs of hex digits joined by `:`,
such as `00:1a:2b:3c:4d:5e`. The second is the same six pairs joined by `-`. The third is three
groups of four hex digits joined by `.`, such as `001a.2b3c.4d5e`. Upper case and lower case
letters are both allowed.

One text uses one separator. A text that mixes `:` and `-` gives `false`. A group with one digit,
a missing group, a space or twelve digits with no separator also give `false`.

**The examples below** check a few texts, show the three ways to write the same address, and
check the addresses in a list of devices.
