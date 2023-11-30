# mini-async-repl [![crates.io](https://img.shields.io/crates/v/mini-async-repl.svg)](https://crates.io/crates/mini-async-repl) [![docs.rs](https://docs.rs/mini-async-repl/badge.svg)](https://docs.rs/mini-sync-repl)

A minimally functional, async-oriented REPL. 

This Rust library is meant to help you build a REPL for your application. This work is a derivative from the great [easy-repl](https://github.com/jedrzejboczar/easy-repl) crate originally authored by [Jędrzej Boczar](https://github.com/jedrzejboczar).

We were actually very happy as users of easy-repl, until we stumbled upon the need to await for async commands. So we decided to fork it and re-design the way commands are handled and re-implement the REPL's loop itself so that it awaitable.

Given the complexities of dealing with Rust's type system, borrow checker, and async programming, we decided to drop the macro oriented approach in easy-repl in favor of a somewhat more complex model. 

All the rest of the high level features available in easy-repl are equally available to mini-async-repl, eg handy help messages, hints and TAB-completion.

While easy-repl automatically handles validation and parsing of params leveraging its macros, we decided to leave those features out for the first version of this crate. Some utilities for validation and parsing are exposed for the library's user to compose their own handlers instead. See the `examples/` directory for more information to learn how to implement different scenarios.

