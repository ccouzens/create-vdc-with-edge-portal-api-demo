const workerConfig = {
  entry: "./src/service_worker.ts",
  output: {
    filename: "serviceWorker.js"
  },
  target: "webworker",
  devtool: "inline-source-map",
  module: {
    rules: [{ test: /\.ts$/, use: "ts-loader", exclude: /node_modules/ }]
  },
  resolve: {
    extensions: [".ts", ".js"]
  }
};

const webConfig = {
  entry: "./src/index.ts",
  output: {
    filename: "main.js"
  },
  target: "web",
  devtool: "inline-source-map",
  module: {
    rules: [{ test: /\.ts$/, use: "ts-loader", exclude: /node_modules/ }]
  },
  resolve: {
    extensions: [".ts", ".js"]
  }
};

module.exports = [workerConfig, webConfig];
