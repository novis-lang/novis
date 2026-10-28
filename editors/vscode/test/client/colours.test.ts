// The rules `nvs.highlighting.distinctNames` writes into the user's settings, held without an editor.
//
// `src/colours.ts` imports no `vscode`. What is pinned is that turning the setting on adds rules for
// the theme showing and keeps every rule the user wrote, that turning it off takes out exactly what
// it wrote and leaves the file as it was, and which theme counts as the one showing.

import * as assert from "node:assert/strict";

import { NOVIS_THEMES, activeTheme, isNovis, rules, withRules, withoutRules } from "../../src/colours";

describe("turning the setting on", () => {
  it("adds the rules for the theme showing, in that theme's block", () => {
    assert.deepEqual(withRules(undefined, "Some Dark", "dark"), { "[Some Dark]": { rules: rules("dark") } });
  });

  it("keeps a rule the user wrote for the same token, and every other key", () => {
    const current = {
      enabled: true,
      "[Some Dark]": { enabled: true, rules: { "class:nvs": "#FF0000", "variable:nvs": "#00FF00" } },
      "[Other]": { rules: { "method:nvs": "#123456" } },
    };
    const next = withRules(current, "Some Dark", "dark");
    assert.deepEqual(next["[Other]"], current["[Other]"]);
    assert.equal(next.enabled, true);
    const block = next["[Some Dark]"] as { enabled: boolean; rules: Record<string, unknown> };
    assert.equal(block.enabled, true);
    assert.equal(block.rules["class:nvs"], "#FF0000");
    assert.equal(block.rules["variable:nvs"], "#00FF00");
    assert.equal(block.rules["method:nvs"], rules("dark")["method:nvs"]);
  });

  it("writes the light palette for a light theme", () => {
    const next = withRules({}, "Some Light", "light");
    assert.deepEqual(next["[Some Light]"], { rules: rules("light") });
  });
});

describe("turning the setting off", () => {
  it("returns the settings to how they were before it was turned on", () => {
    const before = { "[Other]": { rules: { "method:nvs": "#123456" } }, enabled: true };
    const on = withRules(withRules(before, "Some Dark", "dark"), "Some Light", "light");
    assert.deepEqual(withoutRules(on), before);
  });

  it("leaves nothing when nothing else was there", () => {
    assert.equal(withoutRules(withRules(undefined, "Some Dark", "dark")), undefined);
  });

  it("keeps a rule the user changed after the setting wrote it", () => {
    const on = withRules(undefined, "Some Dark", "dark");
    (on["[Some Dark]"] as { rules: Record<string, unknown> }).rules["class:nvs"] = "#FF0000";
    assert.deepEqual(withoutRules(on), { "[Some Dark]": { rules: { "class:nvs": "#FF0000" } } });
  });
});

describe("the theme showing", () => {
  it("is workbench.colorTheme, or the preferred theme for the shade when the editor follows the system", () => {
    const settings = { colorTheme: "Chosen", preferredDark: "Night", preferredLight: "Day" };
    assert.equal(activeTheme(settings, "dark"), "Chosen");
    assert.equal(activeTheme({ ...settings, autoDetect: true }, "dark"), "Night");
    assert.equal(activeTheme({ ...settings, autoDetect: true }, "light"), "Day");
  });

  it("needs no rules when it is a Novis theme", () => {
    assert.ok(isNovis(NOVIS_THEMES.dark));
    assert.ok(isNovis(NOVIS_THEMES.light));
    assert.equal(isNovis("Some Dark"), false);
  });
});
