# cbm8032_to_vulkan

Conversion from CBM 8032 graphical frame data read via serial over USB to a
visualisation using nannou.

## Overview

**[nannou](https://nannou.cc/)** provides the event loop (via winit) and
graphics API (via wgpu). [The nannou guide](https://guide.nannou.cc/) may be of
some help to better understand how nannou projects are structured. The gist is
that:
- The `model` function is called once at the beginning. It does any initial
  setup and returns the initial `Model`, i.e. the user's program state.
- The `update` and `view` functions are then called once per frame
  respectively. `update` updates the state of the `Model`, and `view` presents
  the state of the model to the window. In our case we have two windows, each
  with their own `view` function. One is for the small GUI panel, the other is
  for the main visualisation (`vis` for short-hand).

The **serial.rs** module contains everything related to setting up the serial
port, spawning a dedicated thread for serial communication, and reading/parsing
full `Cbm8032Frame`s. `serial::spawn()` produces a `serial::Handle` which allows
for receiving the most recently read `Cbm8032Frame` in a non-blocking manner via
the `try_recv_frame` method.

The **vis.rs** module defines the `Cbm8032Frame` type along with the graphics
pipeline used for rendering it to the visualisation window via wgpu. The
pipeline uses a buffer of instanced rectangles, where each rectangle has a
unique position (the position of the glyph on the screen) and texture
coordinates (the position within the character sheet from which the glyph should
be sampled). The character sheet can be found at
`./assets/images/PetASCII_Combined.png`.

The **vis.rs** module is also responsible for loading the shaders. The shaders
can be found in `./src/lib/glsl`. More specifically, it is the `*.spv` shaders
that are loaded by the software. Note that these need to be manually re-compiled
from their respective GLSL shaders, as the software won't do this for us. Each
of the GLSL shaders have a comment at the top describing how they can be
compiled to SPIR-V.

The **gui.rs** module is mostly one big `gui::update` function that instantiates
all the widgets for the GUI window in an "immediate mode" manner.

The **conf.rs** module defines a small amount of configuration data that is
loaded and saved when the program is run and closed respectively. The data is
loaded from and saved to `./assets/config.json`.

The `./builds/` and `./build_archive/` is directories for storing backups of
full builds of the executable that were known to work. That said, these may
break if the OS is updated, system dependencies change, etc. I don't exactly
remember why there are two of these dirs! It should be fine to remove these and
setup some other more reliable way of archiving builds persistently if you wish.

## Low-latency serial

It is recommended to have the `setserial` command-line tool installed on
the Linux machine that will run the application. All operating systems seem to
buffer USB serial data by default which is great for higher bandwidth but not
good for low latency. The `setserial` tool allows us to specify low latency as a
priority. The `cbm8032_to_vulkan` software will automatically use the
`setserial` tool (if it's available) to achieve this before opening a serial
port.

## A note on Graphics/Vulkan

Originally nannou used a more direct API around Vulkan, hence the name
`cbm8032_to_vulkan`, though recently switched to [wgpu.rs](https://wgpu.rs/).
WGPU still targets Vulkan on native Linux targets, though will target Metal on
macOS and either DX or Vulkan on Windows (I don't remember exactly which).

**cbm8032_to_vulkan** was developed and tested on Linux, though in theory should
just work on Mac or Windows.
