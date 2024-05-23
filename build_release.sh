#!/usr/bin/env sh
cd webapp
npm install
ng build --configuration production
cd ..
cargo build --package dsbox --release --features embedded_webapp