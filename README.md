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

# PsychoBlend

Included in the repository is an add-on for [Blender](http://www.blender.org)
called "PsychoBlend" that lets you use Psychopath for rendering in Blender.
However, most Blender features are not yet supported because Psychopath itself
doesn't support them yet.

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

See LICENSE.md for details.  But the gist is:

* The overall project is licensed under GPLv3.
* PsychoBlend is licensed under GPLv2, for compatibility with Blender.
* Most crates under the `sub_crates` directory are dual-licensed under MIT and Apache 2.0 (but with some exceptions--see each crate for its respective licenses).

The intent of this scheme is to keep Psychopath itself copyleft, while allowing smaller reusable components to be licensed more liberally.
