If you want to use it with wasm, you need to enter the CORS bypass proxy server in the `PROXY` part of `shared_constants/src/lib.rs`.
And when compiling, you need to enter `RUSTFLAGS=--cfg=web_sys_unstable_apis`.

ex) RUSTFLAGS=--cfg=web_sys_unstable_apis trunk serve
