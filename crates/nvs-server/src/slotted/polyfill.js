function _nvs() {
  for (const t of document.querySelectorAll("template[for]")) {
    const want = '?start name="' + t.getAttribute("for") + '"';
    const walk = document.createTreeWalker(document, 192);
    let start, end, node;
    while ((node = walk.nextNode())) {
      const text = (node.nodeType == 7 ? "?" + node.target + " " + node.data : node.data).trim();
      if (!start) {
        if (text == want) start = node;
      } else if (text == "?end") {
        end = node;
        break;
      }
    }
    t.remove();
    if (start && end) {
      const range = document.createRange();
      range.setStartBefore(start);
      range.setEndAfter(end);
      range.deleteContents();
      range.insertNode(t.content);
    }
  }
}
