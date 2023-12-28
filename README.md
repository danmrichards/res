# RES - Rustendo Entertainment System
A [Nintendo Entertainment System][nes] emulator implemented in [Rust][rust]

## Installation
TBC

## Usage

### Running the emulator
```
Usage: res [OPTIONS] --rom <ROM>

Options:
  -x, --window-w <WINDOW_W>        Width of emulator window [default: 256]
  -y, --window-h <WINDOW_H>        Height of emulator window [default: 240]
  -p, --pixel-scale <PIXEL_SCALE>  Pixel scaling factor [default: 3]
  -r, --rom <ROM>                  path/to/rom
  -h, --help                       Print help
  -V, --version                    Print version
```

### Controls
| Keyboard | NES |
| :------: | :-: |
| Up arrow | D-Pad up |
| Down arrow | D-Pad down |
| Left arrow | D-Pad left |
| Right arrow | D-Pad right |
| Space bar | Select |
| Return | Start |
| A | A |
| S | B |

## Building from source

### Pre-requisites
First install the SDL development libraries. Either with your package manager, example
on Ubuntu:

```shell
$ sudo apt-get install libsdl2-dev
```

Or by visiting the [SDL][sdl] website.

Next you will need to install the Rust toolchain, see the [Rust][rust] website
for instructions.

Lastly, install the [just][just] command runner. This is technically optional,
but it contains shortcuts for building, linting etc that will be referenced
below.

### Building

A normal release build can be created via:

```shell
$ just release
```

A development/debug build can be created via:

```shell
$ just debug
```

The emulator can then be run from the `target/[debug|release]/res` relative to the
root of the repository

[nes]: https://en.wikipedia.org/wiki/Nintendo_Entertainment_System
[rust]: https://www.rust-lang.org/
[sdl]: https://wiki.libsdl.org/SDL2/Installation
[just]: https://github.com/casey/just