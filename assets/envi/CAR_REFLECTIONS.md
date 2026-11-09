# Reflection environment

Studio Small 09 by Sergej Majboroda / Poly Haven, released under CC0.

Source: https://polyhaven.com/a/studio_small_09
License: https://polyhaven.com/license

The source tonemapped panorama was resized to a 2048 x 1024 PNG, with a separate blurred
512 x 256 version for rough material reflections. Downloaded on 2026-11-09.

The car shader uses these maps for approximate image based reflections, together with
direct lights, smooth source normals and the model's material maps. It does not provide
ray traced reflections or full scene reflections.
