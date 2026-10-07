# Porting the Body ECU

The rules of the Rust OpenBSW port (`openbsw/rust/PORTING.md`) apply: a direct translation
of each C++ class with the same behavior, test parity with the upstream tests, no new
failure modes, `// SAFETY:` on every `unsafe`. On top of them:

1. **Ports are `&'static dyn` traits taking `&self`.** The C++ passes its port interfaces
   by reference and shares them between domain modules; the Rust domain modules hold
   `&'static dyn` references and the adapters keep their state in cells. Callbacks
   (`std::function`) are boxed closures capturing the `&'static` controller, which is
   why `init` takes `&'static self`.
2. **Statics, as the C++ objects are.** Every system, adapter, and manager is a static
   in `app/src/lib.rs`, wired in `rust_main` in the order `main.cpp` builds them.
   Configurations have a `DEFAULT` constant for that.
3. **Console lines are the C++ lines.** `printk!` in the domain modules and `log_inf!`
   in the application format the same text the C++ prints, so the consoles compare line
   for line (with Zephyr's log timestamps masked).
4. **Properties are the C++ properties.** The Forkpoint assertions and events keep the
   C++ messages, so both images report the same catalog ids.
5. **Zephyr through the shim only.** A new Zephyr call is one `cpp_` function in
   `app/src/zephyr_shim.c` (marshal the arguments, make the call), one declaration and
   safe wrapper in `ffi/src/target.rs`, and one stub in `ffi/src/host.rs`.
6. **Compile-time choices stay compile-time.** `CONFIG_ADC` and `CONFIG_CAN` are the
   `adc` and `can` features, `HAS_OPENSOMEIP` the `transport` feature, and
   `BODY_ECU_FORKPOINT` the `forkpoint` feature.
