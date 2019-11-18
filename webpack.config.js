module.exports = {
  entry: {
    main: "./src/index.ts",
    serviceWorker: "./src/service_worker.ts"
  },
  devtool: "inline-source-map",
  module: {
    rules: [{ test: /\.ts$/, use: "ts-loader", exclude: /node_modules/ }]
  },
  resolve: {
    extensions: ["ts"]
  }
};
