# Ultimine for Horizon (native mod, `.hmod`)

Port of the idea of FTB Ultimine: hold **sneak** while breaking a block and the
connected blocks (or a tunnel / square) break with it, as the player's own
breaks (tool wear, loot, XP and exhaustion stay vanilla).

Shapes: `shapeless`, `small_tunnel`, `large_tunnel`, `small_square`,
`large_square`. Choose with `HORIZON_ULTIMINE_SHAPE` (default `shapeless`).
Limit: 64 blocks per break.

## Status

- `src/engine.rs`: shape planning, **unit-tested** (9 tests pass).
- `src/lib.rs`: the hook glue. **Not compiled against Horizon yet.** Lines
  marked `// VERIFY` use API names inferred from the host trait.

## Build

    cargo horizon-mod new ultimine --template native   # writes the exact pins + SDK
    # copy src/engine.rs and src/lib.rs over the scaffold's src/
    cargo horizon-mod check
    horizon-launcher native-build . --instance "My Instance" --trust-native-code
    cargo horizon-mod build    # -> target/horizon-mod/ultimine.hmod

Current Horizon packs native mods as `.hmod` (the old `.hnmod` is being merged
into it), so the output file is `ultimine.hmod`.
