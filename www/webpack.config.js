const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require("path");

module.exports = (env) => {
    console.log("env.production", env);
    return {
        entry: "./bootstrap.ts",
        output: {
            path: path.resolve(__dirname, "dist"),
            filename: "bootstrap.js",
        },
        mode: env.production ? "production" : "development",
        module: {
            rules: [
                {
                    test: /\.tsx?$/,
                    use: "ts-loader",
                    exclude: /node_modules/,
                },
            ],
        },
        devServer: {
            contentBase: path.resolve(__dirname, "dist"),
        },
        resolve: {
            extensions: [".tsx", ".ts", ".js"],
        },
        plugins: [new CopyWebpackPlugin({ patterns: ["index.html"] })],
        experiments: {
            syncWebAssembly: true,
        },
    };
};
