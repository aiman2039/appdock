# AppDock logo

`appdock.png` is the selected blue Dock logo, based on
`output/logo-variants/01-dock.png`. The generation and refinement prompts are in
`output/logo-variants/01-dock-prompt.txt`.

The source PNG has a rounded white tile with transparent corners, a 6% canvas
inset, and a corner radius of 22% of the tile width. The blue artwork is unchanged.

`scripts/package.sh` creates the standard macOS icon sizes from this source and
embeds `AppDock.icns` in the application bundle. Both debug and release bundles
use this icon.

The executable also embeds the PNG and sets its application icon at startup,
so direct launches such as `cargo run` use the same logo without a bundle.
