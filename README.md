# NESOxide
A [Nintendo Entertainment System][nes] emulator implemented in [Rust][rust]

## Installation
TBC

## Usage

### Running the emulator
```
Usage: nesoxide [OPTIONS] --rom <ROM>

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
First install SDL development libraries. Either with your package manage, example
on Ubuntu:

```bash
$ sudo apt-get install libsdl2-dev
```

Or by visiting the [SDL][sdl] website.

Next you will need to install the Rust toolchain, see the [Rust][rust] website
for instructions.

### Building

Build using cargo:

```
cargo build -r
```

The emulator can then be run from the `target/release/nesoxide` relative to the
root of the repository

[nes]: https://en.wikipedia.org/wiki/Nintendo_Entertainment_System
[rust]: https://www.rust-lang.org/
[sdl]: https://www.libsdl.org/