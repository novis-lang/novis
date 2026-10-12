// VS Code's HTML and CSS custom data for `Novis\Pdf`, made from `extensions/pdf/subset.json`
// (`rule:core-classes/pdf-css-subset-table`). `bun nv pdf-custom-data` writes the two files under
// `data/` with it, and the contributions suite holds them equal to what it returns, so the subset
// table is the one home of every name and hover.
//
// Only the entries the table marks `"editor": true` are carried: what VS Code's own data lacks.
// Its CSS service keeps the first entry of a name across providers, its own first, so a second
// `content` or `position` would change nothing; its HTML service lists a tag every provider
// declares once per provider, and hovers only the first. So the HTML file is global attributes
// alone, shown in every HTML region because the editor cannot know which markup becomes a PDF,
// and the subset's value functions are carried by neither file: the format has no place for a
// function, and VS Code's CSS linter does not mark an unknown one.
//
// Pure: it reads nothing and writes nothing, so the test and the command run the same code.

interface Entry {
  name: string;
  editor?: boolean;
  description: string;
  example?: string;
  element?: string;
  values?: string[];
}

export interface Subset {
  properties: Entry[];
  atRules: Entry[];
  html: Entry[];
}

export interface CustomData {
  html: string;
  css: string;
}

function markdown(value: string): { kind: "markdown"; value: string } {
  return { kind: "markdown", value };
}

function carried(list: Entry[]): Entry[] {
  return list.filter((entry) => entry.editor === true);
}

// Each file is its JSON with two-space indentation and a final newline.
function text(data: unknown): string {
  return `${JSON.stringify(data, null, 2)}\n`;
}

export function customData(subset: Subset): CustomData {
  const attributes = carried(subset.html).map((entry) => {
    if (entry.element === undefined || entry.element === entry.name) {
      throw new Error(`subset.json: ${entry.name} is marked for the editor but is not an attribute`);
    }
    return {
      name: entry.name,
      description: markdown(entry.description),
      ...(entry.values ? { values: entry.values.map((name) => ({ name })) } : {}),
    };
  });
  const properties = carried(subset.properties).map((entry) => ({
    name: entry.name,
    description: markdown(
      entry.example === undefined
        ? entry.description
        : `${entry.description}\n\n\`${entry.name}: ${entry.example}\``,
    ),
  }));
  const atDirectives = carried(subset.atRules).map((entry) => ({
    name: entry.name,
    description: markdown(entry.description),
  }));
  return {
    html: text({ version: 1.1, globalAttributes: attributes }),
    css: text({ version: 1.1, properties, atDirectives }),
  };
}
