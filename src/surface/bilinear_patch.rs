use super::{point_order, PointOrder, Splitable, MAX_EDGE_DICE};
use crate::{lerp::lerp, math::Point};

#[derive(Debug, Copy, Clone)]
pub struct BilinearPatch<'a> {
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

fn bilerp_point(patch: [Point; 4], uv: (f32, f32)) -> Point {
    let a = lerp(patch[0], patch[1], uv.0);
    let b = lerp(patch[3], patch[2], uv.0);
    lerp(a, b, uv.1)
}

#[derive(Debug, Copy, Clone)]
pub struct BilinearSubPatch<'a> {
    original: &'a BilinearPatch<'a>,
    clip: [(f32, f32); 4],
    must_split: [bool; 4],
}

impl<'a> Splitable for BilinearSubPatch<'a> {
    fn split<F>(&self, metric: F) -> Option<(Self, Self)>
    where
        F: Fn(Point, Point) -> f32,
    {
        // Get the points of the sub-patch.
        let patch = self.original.control_points[0];
        let points = [
            bilerp_point(patch, self.clip[0]),
            bilerp_point(patch, self.clip[1]),
            bilerp_point(patch, self.clip[2]),
            bilerp_point(patch, self.clip[3]),
        ];

        // Calculate edge metrics.
        let edge_metric = [
            metric(points[0], points[1]),
            metric(points[1], points[2]),
            metric(points[2], points[3]),
            metric(points[3], points[0]),
        ];

        // Do the split, if needed.
        for i in 0..4 {
            if self.must_split[i] || edge_metric[i] > MAX_EDGE_DICE as f32 {
                let edge_1 = (i, (i + 1) % 4);
                let edge_2 = ((i + 2) % 4, (i + 3) % 4);
                let new_must_split = {
                    let mut new_must_split = self.must_split;
                    new_must_split[edge_1.0] = false;
                    new_must_split[edge_2.0] = false;
                    new_must_split
                };

                let midpoint_1 = lerp(self.clip[edge_1.0], self.clip[edge_1.1], 0.5);
                let midpoint_2 = {
                    let alpha = if self.must_split[edge_2.0]
                        || edge_metric[edge_2.0] > MAX_EDGE_DICE as f32
                    {
                        0.5
                    } else {
                        let edge_2_dice_rate = edge_metric[edge_2.0].ceil();
                        (edge_2_dice_rate * 0.5).floor() / edge_2_dice_rate
                    };

                    match point_order(points[edge_2.0], points[edge_2.1]) {
                        PointOrder::AsIs => lerp(self.clip[edge_2.0], self.clip[edge_2.1], alpha),
                        PointOrder::Flip => lerp(self.clip[edge_2.1], self.clip[edge_2.0], alpha),
                    }
                };

                // Build the new sub-patches
                let mut patch_1 = BilinearSubPatch {
                    original: self.original,
                    clip: self.clip,
                    must_split: new_must_split,
                };
                let mut patch_2 = patch_1;
                patch_1.clip[edge_1.1] = midpoint_1;
                patch_1.clip[edge_2.0] = midpoint_2;
                patch_2.clip[edge_1.0] = midpoint_1;
                patch_2.clip[edge_2.1] = midpoint_2;

                return Some((patch_1, patch_2));
            }
        }

        // No splitting needed to be done.
        None
    }
}
