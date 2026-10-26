// The viewer's entry point: mounts the app into the page `nvs debug` serves, whose one element is
// `<div id="app">`.

import { render } from "solid-js/web";
import { App } from "./app.tsx";
import "./style.css";

const root = document.getElementById("app");
if (!root) throw new Error("viewer: the page has no #app element");
render(() => <App />, root);
