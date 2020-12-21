# Overview

Psychopath is a path tracing 3d renderer.  You can read about its development
at [psychopath.io](http://psychopath.io).

This project is mostly just for me to have fun, learn, and play with ideas in
3d rendering.  I do have vague hopes that it will eventually be useful for real
things, but that's not a hard goal.

Unlike many for-fun 3d rendering projects, Psychopath is being designed with
production rendering in mind.  I think that architecting a renderer to
efficiently handle very large data sets, complex shading, motion blur, color
management, etc. presents a much richer and more challenging problem space to
explore than just writing a basic path tracer.

## Building
Psychopath is written in [Rust](https://www.rust-lang.org), and is pretty
straightforward to build except for its OpenEXR dependency.

If you have OpenEXR 2.2 installed on your system such that pkg-config can find
it, then as long as you have Rust (including Cargo) and a C++ compiler
installed, you should be able to build Psychopath with this command at the
repository root:

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

# PsychoBlend

Included in the repository is an add-on for [Blender](http://www.blender.org)
called "PsychoBlend" that lets you use Psychopath for rendering in Blender.
However, most Blender features are not yet supported because Psychopath itself
doesn't support them yet.

If you have any trouble getting the add-on working, please feel free to file an
issue and I'll try to troubleshoot/fix it as I have time!

## Features Supported
- Polygon meshes.
- Point, area, and sun lamps (exported as sphere, rectangle, and distant disc lights, respectively)
- Simple materials assigned per-object.
- Focal blur / DoF
- Camera, transform, and deformation motion blur
- Exports dupligroups with full hierarchical instancing
- Limited auto-detection of instanced meshes

# Contributing

I'm not looking for contributions right now, and I'm likely to reject pull
requests.  This is currently a solo project and I like it that way.

However, if you're looking for projects _related_ to Psychopath to contribute to,
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
