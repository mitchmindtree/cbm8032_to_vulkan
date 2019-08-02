# cbm8032_to_vulkan

Conversion from CBM 8032 graphical frame data read via serial over USB to a
visualisation using Vulkan.

## Frame Layout

The software expects frame data to be laid out within 52 buffers of 41
characters.

```
Buffer 0:           [0..                  , 51]
Buffer n 1...50:    [graphics_data..      ,  n]
Buffer 51:          [graphics_state, 0..  , 51]
```
