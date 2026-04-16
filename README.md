# Tile Picker

Game dev tool to help with viewing tilesheets and getting the index of a given
tile. Built with Rust, raylib-rs, and raygui.

## Configuration

Tile Picker optionally reads a `tp.toml` file from the current directory. If the
file isn't present, defaults are used.

```toml
dir = "assets"          # directory to read images from (default: "assets")
one_based_index = true  # use 1-based indexes, useful for Lua projects (default: false)
```

A directory passed as a CLI argument takes precedence over the `dir` config
value.

## Developing

1. Install `just`
2. Run with `just run`

Run all checks with `just ok`

## License

This software is dedicated to the public domain.

Assets in ./assets are made by [Kenney](https://kenney.nl) and also public
domain.
