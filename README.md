<h1 align="center">EVILEFFECTS</h1>

<p align="center">
  <b>Heavy-hitting effect plug-ins for <a href="https://github.com/storytold/effectcraft">EffectCraft</a>, the open-source motion graphics app.</b>
</p>

<p align="center">
  <a href="https://discord.gg/AGfaBwNKfN"><img src="https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  <a href="https://www.youtube.com/@EvelopmentGames"><img src="https://img.shields.io/badge/YouTube-Eve.Lopment%20Games-FF0000?logo=youtube&logoColor=white" alt="YouTube"></a>
  <a href="https://www.instagram.com/evelopmentgames/"><img src="https://img.shields.io/badge/Instagram-evelopmentgames-E4405F?logo=instagram&logoColor=white" alt="Instagram"></a>
  <a href="http://onee.page/evelopmentgames"><img src="https://img.shields.io/badge/Website-onee.page-8A2BE2" alt="Website"></a>
  <img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue" alt="License: MIT OR Apache-2.0">
</p>

<p align="center">
  <a href="https://ko-fi.com/evelynnevlmg"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Support me on Ko-fi"></a>
</p>

## Effects

They show up in EffectCraft's Effects & Presets panel under **EVILEFFECTS**.

| Effect | What it does |
|---|---|
| Evil Flicker | Random brightness flicker over time: amount, speed, minimum brightness, smooth or hard cuts, seed. |
| Evil Chroma Split | Pushes red outward and pulls blue inward from a centre point for RGB fringing. |
| Evil Glow | Blooms the bright parts of the image in several stacked sizes, with threshold, radius, intensity and colour. |

Coming next: Evil Shake, Evil Warp Transform, Evil Motion Curves blur, Evil Edge Rays and Evil Lens Flare.

## Install

Each effect is one `.wasm` file (EffectCraft plug-in API v1). In EffectCraft choose
**Effect ▸ Load Effect Plug-in…** and pick the file, or drop the files into the `Plug-ins` folder
next to EffectCraft's settings so they load at every start-up. Plug-ins work in the desktop app
and the CLI, not the web app.

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
# → target/wasm32-unknown-unknown/release/evil_*.wasm
cargo test   # native tests for every effect
```

Each plug-in lives in `plugins/<name>`; `common/` holds the plug-in ABI glue and shared pixel
helpers (bilinear sampling, blur, noise).

## Community

- Discord: https://discord.gg/AGfaBwNKfN
- YouTube: https://www.youtube.com/@EvelopmentGames
- Instagram: https://www.instagram.com/evelopmentgames/
- Like the effects? You can buy me a coffee on [Ko-fi](https://ko-fi.com/evelynnevlmg).
- Ideas and bugs: [open an issue](https://github.com/Evelynnsrepos/EVILEFFECTS/issues/new)

## License

MIT OR Apache-2.0, at your option. All effects are original implementations written from scratch;
no third-party plug-in code or presets are included.
