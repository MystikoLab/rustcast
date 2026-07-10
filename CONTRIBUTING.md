# Welcome to the RustCast contributing guide!

Thank you for wanting to contribute to RustCast!

There are 2 areas you can work on:

1. Bug fixes
1. New Features
1. Help people in solving their github issues

For bug fixes, and helping people to solve their github issues: see
[https://github.com/MystikoLab/rustcast/issues] For features, see
[The Planned Features in the README](README.md) or
[The existing feature list](FEATURES.md)

## Code Guidelines:

1. All code must be formatted with `cargo fmt`
1. Code must not be malicious or be intended to harm someones device
1. All features added must work as intended
1. Code must compile...
1. No AI Slop. AI Usage is allowed, but please limit it and ensure its a human
   writing the descriptions and minimally ~75% of the code
1. A video recording / screenshot would be an added bonus in getting your pull
   request merged faster.

## Codebase:

```
.
├── bundling # Bundling related stuff, ignore for the most bit
│   ├── entitlements.plist
│   ├── icon.icns
│   └── Info.plist
├── docs # Website and documentation related stuff. If something new is added to config, then modify this as well before PR-ing
├── Cargo.lock 
├── Cargo.toml
├── CONTRIBUTING.md # Contributing guidelines and codebase structure
├── EXTENSIONS.md   # Discussions about extensions implementation
├── LICENSE.md      # License file
├── README.md       # Readme file
├── FEATURES.md     # List of features currently implemented that should be updated when new 
└── src
    ├── app
    │   ├── apps.rs         # Logic for the "apps" / commands that rustcast can perform
    │   ├── menubar.rs      # All the code related to the tray icon / menu bar icon
    │   ├── tile            # Logic for the tile (rustcast window)
    │   │   ├── elm.rs      # Logic for the elm architecture of the rustcast window (New and View)
    │   │   └── update.rs   # Logic for the updating (elm architecture update) of the rustcast window
    │   └── tile.rs         # Tile definition
    ├── app.rs              # All code related to the app
    ├── calculator.rs       # Calculator logic 
    ├── commands.rs         # Logic for different commands
    ├── clipboard.rs        # Logic for the clipboard history feature of rustcast
    ├── config.rs           # Configuration related stuff
    ├── haptics.rs          # All Haptics related code
    ├── macos.rs            # Macos specific config
    ├── main.rs             # Start app
    └── utils.rs            # Common functions that are used across files
```
