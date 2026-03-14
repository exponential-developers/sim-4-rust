# sim-4.0-rust

Theory simulator for the mobile game [Exponential Idle](https://conicgames.github.io/exponentialidle/) based on sim-3.0

This website was originally developed by [XLII](https://github.com/tredec) and now maintained by hotab and Mathis S., with contributions from
the Exponential Idle community.

The site is written in [Rust](https://rust-lang.org/) and [TypeScript](https://www.typescriptlang.org/), built with [webpack](https://webpack.js.org/) and hosted on [GitHub Pages](https://pages.github.com).

# How to use on your computer

## Rust

- Install Rust
- Run `rustup target add wasm32-unknown-unknown` to enable wasm in Rust
- Install `wasm-pack`: `cargo install wasm-pack`
- Make sure you can run `wasm-pack` in your terminal. You might need to add the Cargo bin directory to your terminal `PATH`.


## npm

- Install `npm` on your computer
- Run `npm install` to install the modules
- Run `npm run build` to build the website
- Run `npm run webpack-dev` to build the TypeScript part only, without rebuilding the wasm module
