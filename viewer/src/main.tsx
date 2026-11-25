// The viewer's entry point: mounts the app into the page `nvs debug` serves, whose one element is
// `<div id="app">`.

import { render } from "solid-js/web";
import { App } from "./app.tsx";
// The stylesheet, layer by layer. Bun bundles every import into the one `viewer.css`.
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/ui.css";
import "./styles/requests.css";
import "./styles/request.css";
import "./styles/request-tabs.css";
import "./styles/timeline.css";
import "./styles/value-tree.css";
import "./styles/stream.css";
import "./styles/search.css";
import "./styles/compare.css";
import "./styles/replay.css";
import "./styles/exceptions.css";
import "./styles/editor.css";
import "./styles/coverage.css";
import "./styles/routes.css";
import "./styles/statements.css";
import "./styles/settings.css";

const root = document.getElementById("app");
if (!root) throw new Error("viewer: the page has no #app element");
render(() => <App />, root);
