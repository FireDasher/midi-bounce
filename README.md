# Midi Bounce
Bouncing square video, Rust edition

It is made with [WGPU](https://wgpu.rs/) for rendering and [Rodio](https://docs.rs/rodio/latest/rodio/) for audio, some other libraries like [Winit](https://docs.rs/winit/latest/winit/) for windowing and [glam](https://docs.rs/glam/latest/glam/) for math are used too.

Credits to [Quasar098](https://github.com/quasar098/) for the original [Midi Playground](https://github.com/quasar098/midi-playground)

# Usage
Run it from the command line:  `midi-bounce.exe converted_midi_file.bin audio_file.ogg`  
To convert a midi file, use `midiconverter.py`, it will open a file dialog to open the midi file and save the bin file.
- R: reset
- G: re-generate

# Building
1. Download [Rust](https://rust-lang.org/tools/install/)
2. Clone this repository
3. Use `cargo run` or `cargo build` or `cargo build --release`

# For content creators
If you use this in a video you should put a link to this repository