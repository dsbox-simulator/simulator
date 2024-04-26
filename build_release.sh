#!/usr/bin/env sh
cd webapp
npm install
npm run build -- --env profile=release
cd ..
cargo build --workspace --release