const path = require("path");

module.exports = {
  experiments: {
    asyncWebAssembly: true,
  },
  
  entry: {
    main: [
      "./src/Sim/main.ts",
      "./src/Sim/write.ts",
      "./src/UI/buttonEvents.ts",
      "./src/UI/render.ts",
      "./src/UI/settings.ts",
      "./src/UI/simState.ts"
    ],
  },
  output: {
    filename: "bundle.js",
    // The worker (and its wasm) get a content hash in their names, so browsers load the new ones
    // after a change instead of a cached copy (a hard refresh does not reload worker scripts)
    chunkFilename: "[name].[contenthash].js",
    path: path.resolve(__dirname, 'build'),
    // Remove the files of older builds
    clean: true,
  },
  resolve: {
    extensions: ['.ts', '.js']
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        loader: 'ts-loader',
        exclude: /node_modules/,
      },
      {
        test: /\.wasm$/,
        type: "webassembly/async"
      }
    ]
  }
};
