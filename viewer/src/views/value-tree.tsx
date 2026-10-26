// A dumped value as a tree, in the WAI-ARIA tree pattern: arrow keys move between the nodes, Right and
// Left open and close one, Home and End go to the first and the last, and Enter opens a repeat's node or
// opens and closes a node with children.
//
// The tree arrives in the JSON rendering of a value (`crates/nvs-render/src/json.rs`, § *What a node is
// spelled as*), already bounded: the record cut what was too deep or too long and turned every repeat
// into the `$id` of an instance. So every node is built from data already loaded, and a repeat
// (`$cycle`) is an in-page link to the node with that `$id` in the same tree. A repeat whose `$id` the
// tree does not have is text without a link.
//
// Every value reaches the page as a text node. No value is markup, a link URL, or an attribute other
// than `title`; a link's `href` is built from the node's own DOM id.
//
// What it spends: one object and one open signal per node, for a tree the record already bounded.

import { createSignal, For, Show, type Accessor, type Setter } from "solid-js";
import type { Json } from "../api.ts";
import { ignored } from "../keys.ts";

/** One node of the tree, built once from the JSON. */
interface TreeNode {
  /** The DOM id of its `treeitem`. */
  id: string;
  parent: TreeNode | null;
  level: number;
  /** The property name, the map key or the list position it has in its parent. */
  key: string | null;
  /** What the row shows for the value. */
  text: string;
  /** A muted note after `text`, such as an instance's number or what was cut. */
  note: string;
  /** Which class colours the value. */
  tone: "str" | "num" | "lit" | "class" | "secret" | "cut" | "plain";
  children: TreeNode[];
  /** A repeat's `$id`. */
  repeat?: number;
  /** An instance's `$id`. */
  instance?: number;
  open?: Accessor<boolean>;
  setOpen?: Setter<boolean>;
}

type JsonObject = { [key: string]: Json };

const TAGS = new Set(["$bytes", "$decimal", "$float", "$enum", "$callable", "$redacted", "$elided", "$cycle", "$span"]);

function isObject(value: Json): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function plural(n: number, one: string, many: string): string {
  return n === 1 ? `1 ${one}` : `${n} ${many}`;
}

/** The text of a scalar the JSON rendering writes as a string, a number or an object. */
function scalarText(value: Json): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean" || value === null) return String(value);
  return JSON.stringify(value);
}

/** How many entries a container has in all, counting the ones an `entries` cut left out. */
function total(children: Json[]): number {
  const last = children.at(-1);
  if (last !== undefined && isObject(last) && isObject(last.$elided ?? null)) {
    const cut = last.$elided as JsonObject;
    if (cut.cut === "entries" && typeof cut.total === "number") return cut.total;
  }
  return children.length;
}

/** Builds the nodes of `value` and gives the root. `prefix` keeps the DOM ids of two trees apart. */
function build(value: Json, prefix: string, instances: Map<number, TreeNode>): TreeNode {
  let counter = 0;
  const make = (json: Json, key: string | null, parent: TreeNode | null): TreeNode => {
    const node: TreeNode = {
      id: `${prefix}-n${counter++}`,
      parent,
      level: parent ? parent.level + 1 : 1,
      key,
      text: "",
      note: "",
      tone: "plain",
      children: [],
    };
    const container = (entries: [string, Json][]) => {
      node.children = entries.map(([k, v]) => make(v, k, node));
      const [open, setOpen] = createSignal(true);
      node.open = open;
      node.setOpen = setOpen;
    };

    if (json === null || typeof json === "boolean") {
      node.text = String(json);
      node.tone = "lit";
    } else if (typeof json === "number") {
      node.text = String(json);
      node.tone = "num";
    } else if (typeof json === "string") {
      node.text = `"${json}"`;
      node.tone = "str";
    } else if (Array.isArray(json)) {
      const n = total(json);
      node.text = n === 0 ? "empty list" : `list, ${plural(n, "item", "items")}`;
      if (json.length > 0) container(json.map((v, i) => [String(i), v]));
    } else if ("$class" in json) {
      node.text = String(json.$class);
      node.tone = "class";
      if (typeof json.$id === "number") {
        node.instance = json.$id;
        node.note = `#${json.$id}`;
        instances.set(json.$id, node);
      }
      const props = isObject(json.$properties ?? null) ? Object.entries(json.$properties as JsonObject) : [];
      if (props.length > 0) container(props);
      else node.note = `${node.note} no properties`.trim();
    } else {
      const keys = Object.keys(json);
      const tag = keys.length === 1 && TAGS.has(keys[0]!) ? keys[0]! : null;
      const inner = tag ? json[tag]! : null;
      switch (tag) {
        case "$redacted":
          node.text = "Secret. The value is hidden.";
          node.tone = "secret";
          break;
        case "$cycle":
          node.repeat = typeof inner === "number" ? inner : -1;
          node.tone = "class";
          break;
        case "$elided":
          describeCut(node, isObject(inner) ? inner : {});
          break;
        case "$bytes":
          node.text = scalarText(inner);
          node.note = "bytes, in hex";
          node.tone = "str";
          break;
        case "$decimal":
        case "$float":
          node.text = scalarText(inner);
          node.tone = "num";
          break;
        case "$enum":
          node.text = scalarText(inner);
          node.tone = "lit";
          break;
        case "$callable": {
          const n = isObject(inner) && typeof inner.parameters === "number" ? inner.parameters : 0;
          node.text = `function with ${plural(n, "parameter", "parameters")}`;
          node.tone = "lit";
          break;
        }
        case "$span": {
          const span = isObject(inner) ? inner : {};
          node.text = `${scalarText(span.file ?? "")}:${scalarText(span.line ?? "")}:${scalarText(span.column ?? "")}`;
          node.note = typeof span.label === "string" ? span.label : "";
          break;
        }
        default: {
          const entries = Object.entries(json);
          const n = total(entries.map(([, v]) => v));
          node.text = n === 0 ? "empty map" : `map, ${plural(n, "entry", "entries")}`;
          if (entries.length > 0) container(entries);
        }
      }
    }
    return node;
  };
  return make(value, null, null);
}

/** What an `$elided` node says was cut, and how much. */
function describeCut(node: TreeNode, cut: JsonObject) {
  node.tone = "cut";
  if (cut.cut === "entries") {
    const n = typeof cut.entries === "number" ? cut.entries : 0;
    const all = typeof cut.total === "number" ? cut.total : n;
    node.text = `${n} of ${all} entries are not recorded.`;
  } else if (cut.cut === "text") {
    const bytes = typeof cut.bytes === "number" ? cut.bytes : 0;
    node.text = `"${typeof cut.kept === "string" ? cut.kept : ""}"`;
    node.note = `${plural(bytes, "more byte is", "more bytes are")} not recorded.`;
  } else {
    node.text = "Not recorded. This value is nested too deeply.";
  }
}

/** A dumped value as a keyboard-navigable tree. `name` is the tree's accessible name. */
export function ValueTree(props: { value: Json; prefix: string; name: string }) {
  const instances = new Map<number, TreeNode>();
  const root = build(props.value, props.prefix, instances);
  const byId = new Map<string, TreeNode>();
  const index = (node: TreeNode) => {
    byId.set(node.id, node);
    node.children.forEach(index);
  };
  index(root);

  const [current, setCurrent] = createSignal(root.id);
  const [jumped, setJumped] = createSignal<string | null>(null);
  let tree!: HTMLUListElement;

  const focus = (id: string) => {
    setCurrent(id);
    document.getElementById(id)?.focus();
  };

  /** Opens every node above `target`, focuses it and highlights it. */
  const jump = (target: TreeNode) => {
    for (let up = target.parent; up; up = up.parent) up.setOpen?.(true);
    setJumped(target.id);
    focus(target.id);
  };

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e)) return;
    const items = [...tree.querySelectorAll<HTMLElement>("[role=treeitem]")];
    const at = items.findIndex((item) => item.id === current());
    const node = byId.get(current());
    if (!node) return;
    switch (e.key) {
      case "ArrowDown":
        if (items[at + 1]) focus(items[at + 1]!.id);
        break;
      case "ArrowUp":
        if (at > 0) focus(items[at - 1]!.id);
        break;
      case "Home":
        focus(items[0]!.id);
        break;
      case "End":
        focus(items.at(-1)!.id);
        break;
      case "ArrowRight":
        if (node.open && !node.open()) node.setOpen!(true);
        else if (node.open && node.children[0]) focus(node.children[0].id);
        break;
      case "ArrowLeft":
        if (node.open?.()) node.setOpen!(false);
        else if (node.parent) focus(node.parent.id);
        break;
      case "Enter": {
        const target = node.repeat === undefined ? undefined : instances.get(node.repeat);
        if (target) jump(target);
        else if (node.open) node.setOpen!(!node.open());
        else return;
        break;
      }
      default:
        return;
    }
    e.preventDefault();
  };

  const Item = (p: { node: TreeNode }) => {
    const node = p.node;
    const target = node.repeat === undefined ? undefined : instances.get(node.repeat);
    return (
      <li
        id={node.id}
        role="treeitem"
        aria-level={node.level}
        aria-expanded={node.open ? (node.open() ? "true" : "false") : undefined}
        aria-labelledby={`${node.id}-row`}
        tabindex={current() === node.id ? 0 : -1}
        data-instance={node.instance}
        classList={{ "tv-jumped": jumped() === node.id }}
      >
        <div
          id={`${node.id}-row`}
          class="tv-row"
          onClick={(e) => {
            if (e.defaultPrevented) return;
            focus(node.id);
          }}
        >
          <span class="tv-twisty" aria-hidden="true" onClick={() => node.setOpen?.(!node.open!())}>
            {node.open ? (node.open() ? "▾" : "▸") : ""}
          </span>
          <Show when={node.key !== null}>
            <span class="tv-key">{node.key}</span>
            <span class="tv-colon">: </span>
          </Show>
          <Show
            when={node.repeat !== undefined}
            fallback={<span class={`tv-${node.tone}`}>{node.text}</span>}
          >
            <Show when={target} fallback={<span class="tv-class">Same object as #{node.repeat}</span>}>
              {(to) => (
                <a
                  class="tv-repeat"
                  href={`#${to().id}`}
                  tabindex="-1"
                  onClick={(e) => {
                    e.preventDefault();
                    jump(to());
                  }}
                >
                  Same object as {to().text} #{node.repeat}
                </a>
              )}
            </Show>
          </Show>
          <Show when={node.note}>
            <span class="tv-note"> {node.note}</span>
          </Show>
        </div>
        <Show when={node.open?.()}>
          <ul role="group">
            <For each={node.children}>{(child) => <Item node={child} />}</For>
          </ul>
        </Show>
      </li>
    );
  };

  return (
    <ul ref={tree} class="tree" role="tree" aria-label={props.name} onKeyDown={onKey}>
      <Item node={root} />
    </ul>
  );
}
