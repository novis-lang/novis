// The modules the viewer imports that ship no types. The build passes the two Babel presets to Babel
// and reads nothing else of them.
declare module "babel-preset-solid" {
  const preset: import("@babel/core").PluginItem;
  export default preset;
}
declare module "@babel/preset-typescript" {
  const preset: import("@babel/core").PluginItem;
  export default preset;
}
// A stylesheet is imported for its side effect alone: Bun bundles it into `viewer.css`.
declare module "*.css" {}
