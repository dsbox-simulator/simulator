import * as esbuild from 'esbuild';
import htmlPlugin from '@chialab/esbuild-plugin-html';
import {program} from "commander";

program
    .option("--release")
    .option("--watch")
    .option("--serve")
    .option("--port <number>", undefined, 8000);
program.parse();
const opts = program.opts();


const options = {
    entryPoints: ["index.html"],
    outdir: "dist",
    bundle: true,
    minify: opts.release,
    treeShaking: opts.release,
    format: "esm",
    sourcemap: !opts.release ? 'linked' : false,
    tsconfig: 'tsconfig.json',
    define: {DEV: opts.release ? 'false' : 'true'},
    plugins: [htmlPlugin()],
    loader: {
        ".woff": "file",
        ".woff2": "file",
        ".png": "file",
    }
};

if (!opts.watch && !opts.serve) {
    await esbuild.build(options);
} else {
    const ctx = await esbuild.context(options);
    if (opts.serve) {
        const {hosts, port} = await ctx.serve({servedir: options.outdir, host:"localhost", port:opts.port});
        console.log(`listening on`);
        for(const host of hosts) {
            console.log(`\thttp://${host}:${port}`);
        }
        console.log(`Ctrl+C to stop`);
    } else {
        await ctx.watch();
    }
}