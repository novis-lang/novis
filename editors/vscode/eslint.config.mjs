import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["out/**", "node_modules/**", ".vscode-test/**", "*.vsix"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.ts"],
    rules: {
      "@typescript-eslint/naming-convention": [
        "warn",
        { selector: "import", format: ["camelCase", "PascalCase"] }
      ],
      curly: "warn",
      eqeqeq: "warn",
      "no-throw-literal": "warn",
      semi: "warn"
    }
  },
  {
    // The headless runner is plain Node, so its globals are declared here rather than pulled in
    // as another dependency for three names.
    files: ["scripts/**/*.mjs"],
    languageOptions: {
      sourceType: "module",
      globals: { console: "readonly", process: "readonly" }
    }
  }
);
