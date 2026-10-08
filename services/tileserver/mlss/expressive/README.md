# Headway v3 in Expressive

`headway-v3.exp` is a complete Expressive implementation of the Headway v3
style. It owns draw order and imports four ordered layer modules: `base`,
`land`, `transport`, and `labels`.

Each layer module imports `sources.exp` and spreads its shared source handle
into each layer. `bin/test-expressive-style` compiles the implementation and
asserts exact structural equality with `assets/styles/basic-v3.json`.
