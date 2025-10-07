import * as esbuild from 'esbuild';
import htmlPlugin from '@chialab/esbuild-plugin-html';

const inProduction = process.env.NODE_ENV === "production";

await esbuild.build({
    entryPoints: ["index.html"],
    outdir: "dist",
    bundle: true,
    minify: inProduction,
    treeShaking: inProduction,
    format: "esm",
    sourcemap: !inProduction ? 'linked' : false,
    tsconfig: 'tsconfig.json',
    define: {DEV: inProduction ? 'false' : 'true'},
    plugins: [htmlPlugin()],
    loader: {
        ".woff": "file",
        ".woff2": "file",
    }
});