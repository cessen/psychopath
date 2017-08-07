# Overview

Psychopath is a path tracer, aimed at rendering animations and VFX for
film.  It is currently still in an early prototyping stage of development.

This project is mostly for fun, but I do hope it eventually becomes useful.
That "for-fun" disclaimer aside, the long-term goals of Psychopath are to
support efficient global illumination rendering of scenes that are
significantly larger than available RAM and/or that contain procedural elements
that need to be generated on-the-fly during rendering.

The approach that Psychopath takes to enable this is to try to access the scene
data in as coherent a fashion as possible via breadth-first ray tracing,
allowing the cost of HDD access, expensive procedurals, etc. to be amortized
over large batches of rays.

Psychopath used to be written in C++ but is now written in [Rust](https://www.rust-lang.org).
The curious can take a look at the old C++ code-base [here](https://github.com/cessen/psychopath_cpp).

I occasionally blog about Psychopath's development at [psychopath.io](http://psychopath.io).

## Building
Building Psychopath is mostly straightforward except for its OpenEXR dependency.

If you have OpenEXR 2.2 installed on your system such that pkg-config can find
it, then as long as you have Rust (including Cargo) and a C++ compiler
installed, you should be able to build with this command at the repository
root:

```
cargo build --release
```

However, if you are on an OS that doesn't have pkg-config (e.g. OSX, Windows),
or you prefer to do a custom build of OpenEXR, then you will need to download
and build OpenEXR yourself and specify the necessary environment variables as
documented in the [OpenEXR-rs readme](https://github.com/cessen/openexr-rs/blob/master/README.md).

Once those environment variables are set, then you should be able to build using
the same simple cargo command above.

If you have any difficulties, please feel free to file an issue and I'll try to
help out as I have time!

## Current Features
- Geometry:
  - Triangle meshes (both flat and smooth shading)
- Lights:
  - Spherical light sources
  - Rectangular light sources
  - Distant disc light sources (a.k.a. sun lights)
- Motion blur:
  - Camera motion blur
  - Deformation motion blur
  - Transform motion blur
- Focal blur / DoF
- Spectral rendering (via monte carlo sampling)
- Full hierarchical instancing
- Light Tree sampling for efficient handling of large numbers of lights. (See [this thread](http://ompf2.com/viewtopic.php?f=3&t=1938) for an overview of the technique.)
- Shading:
  - A simple material system that supports single-color Lambert and GTR BRDFs assigned per-instance.

# PsychoBlend

Included in the repository is an addon for [Blender](http://www.blender.org)
called "PsychoBlend" that lets you use Psychopath for rendering in Blender.
However, most Blender features are not yet supported because Psychopath itself
doesn't support them yet.

If you have any trouble getting the addon working, please feel free to file an
issue and I'll try to troubleshoot/fix it as I have time!

## Features Supported
- Meshes (rendered as flat-shaded triangle meshes)
- Point, area, and sun lamps (exported as sphere, rectangle, and distant disc lights, respectively)
- Simple materials assigned per-object.
- Focal blur / DoF
- Camera, transform, and deformation motion blur
- Exports dupligroups with full hierarchical instancing
- Limited auto-detection of instanced meshes

# Contributing

I'm not looking for contributions right now, and I'm likely to reject pull
requests.  This is currently a solo project and I like it that way.  Eventually
when things become less playful/experimental I will likely want to start
collaborating, but that's quite a ways off.

However, I _do_ want people to be able to play with Psychopath, so if you have
trouble getting it to build/run please file an issue!  And if you want to fork
it and play around with the code yourself (or start an entirely new project
based on it!) feel free.  That's why I put it under the MIT license.

Also, if you're looking for projects _related_ to Psychopath to contribute to,
[OpenEXR-rs](https://github.com/cessen/openexr-rs) is definitely a
collaborative project that I would love more help with!  And I fully expect more
such projects to come out of Psychopath in the future.

# License

The original code in Psychopath is distributed under the [MIT license](https://opensource.org/licenses/MIT).

PsychoBlend is distributed under the [GPL version 2](https://opensource.org/licenses/GPL-2.0)
or (at your option) any later version.

Some code in this repository was adapted to Rust from other sources.  With one
exception, all of the adapted code is from sources that are licensed under the
MIT license or a BSD license variant.  Adapted code is marked by comments citing
their source.

The one exception is the code in `sub_crates/spectra_xyz/src/generate_spectra_rust.py`,
which is adapted from the supplemental material of the paper
["Physically Meaningful Rendering using Tristimulus Colours" by Meng et al.](https://cg.ivd.kit.edu/spectrum.php)
It has no explicit license, but I contacted one of the authors and confirmed
that it is intended to be used freely.  Take that for what you will!
