// Bundles sim-3's T1 (unmodified source) for Node so it can be benchmarked against the Rust port.
// Run from the sim-3 checkout so its webpack/ts-loader are used:
//   cd ../sim-3.0 && npx.cmd webpack --config ../sim-4-rust/bench/webpack.sim3.config.js
const path = require("path");
const webpack = require("webpack");

const SIM3 = path.resolve(process.env.SIM3_DIR || path.join(__dirname, "..", "..", "sim-3.0"));

module.exports = {
  mode: "production",
  target: "node",
  context: SIM3,
  entry: path.join(__dirname, "sim3-entry.js"),
  output: {
    path: path.join(__dirname, "out"),
    filename: "sim3-t1.js",
    library: { type: "commonjs2" },
  },
  resolve: {
    extensions: [".ts", ".js"],
    alias: { SIM3: path.join(SIM3, "src") },
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        loader: require.resolve("ts-loader", { paths: [SIM3] }),
        options: { transpileOnly: true, configFile: path.join(SIM3, "tsconfig.json") },
      },
    ],
  },
  plugins: [
    // T1.ts imports `global` from Sim/main.ts, which pulls in the DOM UI. Replace it with a stub.
    new webpack.NormalModuleReplacementPlugin(/[\\/]Sim[\\/]main$/, path.join(__dirname, "sim3-main-stub.js")),
  ],
};
