// The colours Novis gives a type and a method, and the user settings that add them to another theme,
// decided without an editor.
//
// `rule:ide/novis-ships-names-not-colours` keeps every colour the user's: the extension contributes no
// colour default. A user who wants a type and a method told apart has two ways to ask for it. They
// pick one of the two bundled themes, `Novis Dark` or `Novis Light`, or they turn on
// `nvs.highlighting.distinctNames`, which writes the rules below into their user settings, under the
// theme they already use. Both ways use the one palette here, and the contributions suite asserts the
// theme files agree with it. Nothing here imports `vscode`, which is what lets the headless tier hold
// it; `src/theme.ts` is the half that reads and writes the editor's settings.

/** Whether a theme draws light text on a dark background or the reverse. */
export type Shade = "dark" | "light";

/** The two colours a type and a method get, one pair per shade. */
export const PALETTE: Record<Shade, { type: string; method: string }> = {
  dark: { type: "#4EC9B0", method: "#FFC66D" },
  light: { type: "#267F99", method: "#795E26" },
};

/** The label of the bundled theme for each shade, which is also its id in `workbench.colorTheme`. */
export const NOVIS_THEMES: Record<Shade, string> = { dark: "Novis Dark", light: "Novis Light" };

/** A value of `editor.semanticTokenColorCustomizations`, or of one theme's block inside it. */
type Customizations = Record<string, unknown>;

/**
 * The semantic token rules the setting writes for a theme of `shade`. Each is limited to `.nvs`
 * documents by its `:nvs` suffix, so no other language changes colour.
 */
export function rules(shade: Shade): Customizations {
  const { type, method } = PALETTE[shade];
  return {
    "namespace:nvs": type,
    "class:nvs": type,
    "interface:nvs": { foreground: type, italic: true },
    "enum:nvs": type,
    "type:nvs": type,
    "method:nvs": method,
  };
}

/**
 * `current` with the rules for `theme` added. A rule the user already wrote for the same token
 * type stays as they wrote it, so turning the setting on never overwrites their own colour.
 */
export function withRules(current: unknown, theme: string, shade: Shade): Customizations {
  const all = record(current);
  const key = `[${theme}]`;
  const block = record(all[key]);
  return { ...all, [key]: { ...block, rules: { ...rules(shade), ...record(block.rules) } } };
}

/**
 * `current` with every rule the setting wrote taken out of every theme's block, or `undefined` when
 * nothing is left. A rule whose value is not exactly the one the setting writes is the user's, and
 * stays. A block left empty goes too, so turning the setting off returns the file to how it was.
 */
export function withoutRules(current: unknown): Customizations | undefined {
  const written = [rules("dark"), rules("light")];
  const result: Customizations = {};
  for (const [key, value] of Object.entries(record(current))) {
    if (!key.startsWith("[")) {
      result[key] = value;
      continue;
    }
    const block = { ...record(value) };
    const kept = Object.fromEntries(
      Object.entries(record(block.rules)).filter(
        ([token, rule]) => !written.some((ours) => same(ours[token], rule)),
      ),
    );
    delete block.rules;
    if (Object.keys(kept).length > 0) {
      block.rules = kept;
    }
    if (Object.keys(block).length > 0) {
      result[key] = block;
    }
  }
  return Object.keys(result).length > 0 ? result : undefined;
}

/** The theme settings that decide which theme is showing. */
export interface ThemeSettings {
  colorTheme?: string;
  autoDetect?: boolean;
  preferredDark?: string;
  preferredLight?: string;
}

/**
 * The name of the theme that is showing. With `window.autoDetectColorScheme` on, the editor follows
 * the system and shows the preferred theme for its shade; otherwise it shows `workbench.colorTheme`.
 */
export function activeTheme(settings: ThemeSettings, shade: Shade): string | undefined {
  if (settings.autoDetect) {
    return shade === "dark" ? settings.preferredDark : settings.preferredLight;
  }
  return settings.colorTheme;
}

/** Whether `theme` is one of the bundled themes, which have these colours already. */
export function isNovis(theme: string): boolean {
  return Object.values(NOVIS_THEMES).includes(theme);
}

/** Whether two settings values are the same JSON. */
export function same(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function record(value: unknown): Customizations {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Customizations)
    : {};
}
