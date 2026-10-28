// The editor half of `src/colours.ts`: the command that switches to a bundled theme, and the setting
// that writes a type's and a method's colours into the user's settings for the theme they use.
//
// The setting is applied when it changes, when the colour theme changes, and once on activation,
// because it can be changed while no Novis file has woken the extension. The rules always go to the
// user settings, which is where a theme's colours live, so the setting itself is user-scoped. A
// `globalState` flag remembers that rules were written, so a setting turned off while the extension
// slept still removes them, and a user who never turned it on never has a rule of theirs touched.
//
// Every window runs this, and a theme change reaches them all at once. They compute the same value,
// so a write that fails because another window wrote first is dropped. A write the user asked for by
// changing the setting reports its failure.

import { ColorThemeKind, ConfigurationTarget, ExtensionContext, commands, window, workspace } from "vscode";

import { NOVIS_THEMES, Shade, activeTheme, isNovis, same, withRules, withoutRules } from "./colours";

const CUSTOMIZATIONS = "editor.semanticTokenColorCustomizations";

/** The `globalState` key that says this extension has written rules the setting owns. */
const WROTE = "nvs.highlighting.distinctNames.wrote";

export function install(context: ExtensionContext): void {
  context.subscriptions.push(
    commands.registerCommand("nvs.useNovisTheme", () => useNovisTheme()),
    workspace.onDidChangeConfiguration((change) => {
      if (change.affectsConfiguration("nvs.highlighting.distinctNames")) {
        void apply(context, true);
      }
    }),
    window.onDidChangeActiveColorTheme(() => void apply(context, false)),
  );
  void apply(context, false);
}

/** The shade of the theme that is showing. A high-contrast theme counts as the shade it is drawn in. */
function shade(): Shade {
  const kind = window.activeColorTheme.kind;
  return kind === ColorThemeKind.Light || kind === ColorThemeKind.HighContrastLight ? "light" : "dark";
}

/**
 * Switches to the bundled theme for the shade showing now. With the editor following the system's
 * shade, both preferred themes are set instead, so the shade keeps following the system.
 */
async function useNovisTheme(): Promise<void> {
  const settings = workspace.getConfiguration();
  if (settings.get<boolean>("window.autoDetectColorScheme")) {
    await settings.update("workbench.preferredDarkColorTheme", NOVIS_THEMES.dark, ConfigurationTarget.Global);
    await settings.update("workbench.preferredLightColorTheme", NOVIS_THEMES.light, ConfigurationTarget.Global);
    return;
  }
  await settings.update("workbench.colorTheme", NOVIS_THEMES[shade()], ConfigurationTarget.Global);
}

/** Adds the rules for the theme showing, or removes every rule the setting wrote. */
async function apply(context: ExtensionContext, asked: boolean): Promise<void> {
  const on = workspace.getConfiguration("nvs").get<boolean>("highlighting.distinctNames", false);
  const wrote = context.globalState.get<boolean>(WROTE, false);
  if (!on && !wrote) {
    return;
  }
  const settings = workspace.getConfiguration();
  const current = settings.inspect(CUSTOMIZATIONS)?.globalValue;
  let next: unknown = withoutRules(current);
  if (on) {
    const theme = activeTheme(
      {
        colorTheme: settings.get<string>("workbench.colorTheme"),
        autoDetect: settings.get<boolean>("window.autoDetectColorScheme"),
        preferredDark: settings.get<string>("workbench.preferredDarkColorTheme"),
        preferredLight: settings.get<string>("workbench.preferredLightColorTheme"),
      },
      shade(),
    );
    if (theme === undefined || isNovis(theme)) {
      return;
    }
    next = withRules(current, theme, shade());
  }
  try {
    if (!same(next, current)) {
      await settings.update(CUSTOMIZATIONS, next, ConfigurationTarget.Global);
    }
    await context.globalState.update(WROTE, on);
  } catch (error) {
    if (asked) {
      void window.showWarningMessage(`Novis could not change your colour settings: ${String(error)}`);
    }
  }
}
