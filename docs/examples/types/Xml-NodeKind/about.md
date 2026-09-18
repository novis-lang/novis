What a node in a parsed document is: the document itself, an element, some text, a comment or a
processing instruction. There are five and there will never be a sixth, so a walk that handles all
five has handled everything a document can hold.

Reading HTML and reading XML answer the same five kinds, so a walk written for one works unchanged
on the other. Every node
answers which kind it is, and that answer is the first question a walk asks, before it reaches for a
name, a piece of text or a list of children.

**Good to know:** the children of a tag are not all tags. In a file somebody indented by hand, the
line breaks between the tags are text nodes of their own, and a note left in the markup is a comment
node sitting among them. An element also carries no characters itself: the words it wraps are in the
text node underneath it.
