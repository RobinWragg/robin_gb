<div align="center">

# 🦀`robin_gb`🎮
**A crate for Game Boy emulation**

</div>

## Current Status & Goals

223 of the 245 core instructions have been implemented. The main subsystems are up and running, except the foreground tile rendering.

Once actual gameplay is possible, the short-term goal is to build a test harness that will run many instances of the emulator simultaneously to catch bugs and incompatibilities.

Possible longer-term goals:
* Implement novel rendering modes, such as presenting tiles as 3D cubes
* Experiment with content-aware upscaling, especially for text
* Add support for MIDI input and output, to enable chiptune musicians to play the in-game instruments