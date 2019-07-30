use super::Splitable;
use crate::math::Point;

#[derive(Debug, Copy, Clone)]
pub struct BilinearPatch<'a> {
    time_sample_count: usize,

    // The control points are stored in clockwise order, like this:
    //  u ----->
    // v  0  1
    // |  3  2
    // \/
    control_points: &'a [[Point; 4]],

    // Indicates if any of the edges *must* be split, for example if there
    // are adjacent patches that were split for non-dicing reasons.
    //
    // Matching the ascii graph above, the edges are:
    //
    //      0
    //   -------
    // 3 |     | 1
    //   -------
    //      2
    must_split: [bool; 4],
}

#[derive(Debug, Copy, Clone)]
pub struct BilinearSubPatch<'a> {
    original: &'a BilinearPatch<'a>,
    clip: [(f32, f32); 4],
    must_split: [bool; 4],
}

impl<'a> Splitable for BilinearSubPatch<'a> {
    fn split(&self /* TODO: splitting criteria. */) -> Option<(Self, Self)> {
        unimplemented!()
    }
}
