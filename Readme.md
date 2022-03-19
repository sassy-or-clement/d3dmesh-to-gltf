# d3dmesh-to-gltf

This project provides a stand-alone program for converting `*.d3dmesh`, `.d3dtx` and `*.skl` from Telltales *The Walking Dead: The Telltale Definitive Series* (TTDS) into valid [glTF 2.0](https://github.com/KhronosGroup/glTF) and PNG files, which can be read by various 3D programs, including Blender.
This application is based on Random Talking Bushs [Telltale Games "Almost-All-In-One" Model Importer](https://forum.xentax.com/viewtopic.php?f=16&t=11687&sid=6f8042ba574b8db30c500fe4520a66fc).

## Limitations

This application currently has no graphical user interface, meaning it needs to be operated on the command line.
Support for other operating systems than Windows is possible, but case-sensitive file systems (e.g. most Linux distributions) are not currently supported.

It only supports version 55 of these file formats, i.e. it is limited to *The Walking Dead: The Telltale Definitive Series* and possibly *The Walking Dead: The Final Season* (untested).

Although the glTF format is widely supported, this application is mainly aimed at providing suitable files for importing them into Blender.
Some textures contain alpha channels and those are not supported by default, e.g. hair textures need to be manually set up.

There are still issues with certain files of TTDS, ranging from missing string-CRC64 mappings or unrecognized formats.
Please open up an issue when such an issue was encountered.

## Usage

Either download the [latest executable](https://github.com/sassy-or-clement/d3dmesh-to-gltf/releases/latest/download/d3dmesh-to-gltf.exe) (triggers anti-virus warnings) or compile it yourself.
[Rustup](https://www.rust-lang.org/tools/install) is required for compiling and the compilation can be started by executing `cargo build --release` in the root directory of this repository.
The final executable can be found in `target\release\d3dmesh-to-gltf.exe`.

Execute `d3dmesh-to-gltf.exe -h` to print the command reference.

By default, the folder `input` lying next to the executable is searched for `*.d3dmesh`, `.d3dtx` and `*.skl` files (without any directories).
Ideally, all files inside this directory originate from the same archive file (e.g. `WDC_pc_WalkingDead404_txmesh.ttarch2` (`*.d3dmesh` and `*.d3dtx`) and `WDC_pc_WalkingDead404_data.ttarch2` (`*.skl`)).
This program can not convert `*.ttarch2` files, but other programs like the [Telltale Explorer](https://quickandeasysoftware.net/software/telltale-explorer) can do the job.
Note that the referenced textures can be stored in different `.ttarch2` files, e.g. a `.d3dmesh` in `WDC_pc_WalkingDead101_txmesh.ttarch2` might reference textures in `WDC_pc_WalkingDead203_txmesh.ttarch2`.

The output is written by default to the folder `output`, which will contain a pair of `*.gltf` and `*.bin` files for each `*.d3dmesh` and `*.skl` file in the input folder.
The textures are placed as `*.png` in the `output/textures` folder.
Note that the textures are **referenced** by the `*.gltf` files, i.e. the `textures` folder always needs to reside next to these files.
Additionally, log files (`*.log`) are placed into the output folder as well.
These can be useful when errors were printed to the console during execution and provide further detail.

Note: although this program is heavily multi-threaded, converting the files form a single `*.ttarch2` file can take a *long* time.
This mainly boils down to the texture conversion steps and PNG-compression.
For this reason, the automatic generation of height maps from normal maps is disabled by default.
This generation approximates a height map by using a normal map as a starting point.
The implementation is based of [this paper](https://doi.org/10.1145/2037826.2037839).

## Acknowledgements

The basis of this implementation is the [Telltale Games "Almost-All-In-One" Model Importer](https://forum.xentax.com/viewtopic.php?f=16&t=11687&sid=6f8042ba574b8db30c500fe4520a66fc) by Random Talking Bush and the string-CRC64 mappings are re-used in this project.
