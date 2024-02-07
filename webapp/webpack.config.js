const path = require('path');
const autoprefixer = require('autoprefixer');
const HtmlWebpackPlugin = require('html-webpack-plugin')

module.exports = (env) => {
    let webpack_config = {
        mode: env.profile === 'release' ? 'production' : 'development',
        entry: './src/ts/index.tsx',
        plugins: [
            new HtmlWebpackPlugin({ template: './src/index.html' })
        ],
        output: {
            filename: 'app.js',
            path: path.resolve(__dirname, 'dist'),
            library: 'App',
        },
        resolve: {
            extensions: ['.tsx', '.ts', '.js'],
        },
        module: {
            rules: [
                {
                    test: /\.tsx?$/,
                    use: 'ts-loader',
                    exclude: /node_modules/,
                },
                {
                    test: /\.(scss)$/,
                    use: [
                        {
                            // Adds CSS to the DOM by injecting a `<style>` tag
                            loader: 'style-loader'
                        },
                        {
                            // Interprets `@import` and `url()` like `import/require()` and will resolve them
                            loader: 'css-loader'
                        },
                        {
                            // Loader for webpack to process CSS with PostCSS
                            loader: 'postcss-loader',
                            options: {
                                postcssOptions: {
                                    plugins: [
                                        autoprefixer
                                    ]
                                }
                            }
                        },
                        {
                            // Loads a SASS/SCSS file and compiles it to CSS
                            loader: 'sass-loader'
                        }
                    ]
                }
            ],
        },
    };

    if (env.profile !== 'release') {
        webpack_config.devtool = 'source-map';
    }

    return webpack_config;
};