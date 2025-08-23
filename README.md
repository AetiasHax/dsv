# dsv
Structured memory viewer for DS games using types from C/C++ headers

## Contents
- [How to use](#how-to-use)
- [Supported games](#supported-games)

## How to use

1. Install Clang 20+
2. Download dsv from the [Releases page](https://github.com/AetiasHax/dsv/releases/latest)
3. Download melonDS from [this fork](https://github.com/AetiasHax/melonDS/actions?query=branch%3Aci%2Fgdb)
    - If your OS has no prebuilt binaries, clone the fork and run it locally
4. In melonDS, enable GDB stub
    - <kbd>Config</kbd> -> <kbd>Emu settings</kbd> -> <kbd>Devtools</kbd> -> <kbd>Enable GDB stub</kbd>
5. Start a game in melonDS
6. In dsv, configure your dsv project
7. Press <kbd>Connect</kbd> to connect to melonDS and <kbd>Load types</kbd> to scan for C/C++ headers in the project path you provided

## Supported games
For now, dsv only supports *The Legend of Zelda: Phantom Hourglass* and *The Legend of Zelda: Spirit Tracks*. Support for any game is planned!
