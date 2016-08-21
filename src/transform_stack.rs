use std::cmp;

use algorithm::merge_slices_to;
use math::Matrix4x4;


pub struct TransformStack {
    stack: Vec<Matrix4x4>,
    stack_indices: Vec<usize>,
}

impl TransformStack {
    pub fn new() -> TransformStack {
        let mut ts = TransformStack {
            stack: Vec::new(),
            stack_indices: Vec::new(),
        };

        ts.stack_indices.push(0);
        ts.stack_indices.push(0);

        ts
    }

    pub fn clear(&mut self) {
        self.stack.clear();
        self.stack_indices.clear();
        self.stack_indices.push(0);
        self.stack_indices.push(0);
    }

    pub fn push(&mut self, xforms: &[Matrix4x4]) {
        assert!(xforms.len() > 0);

        if self.stack.len() == 0 {
            self.stack.extend(xforms);
        } else {
            let sil = self.stack_indices.len();
            let i1 = self.stack_indices[sil - 2];
            let i2 = self.stack_indices[sil - 1];
            // Reserve stack space for the new transforms.
            // Note this leaves exposed uninitialized memory.  The subsequent call to
            // merge_slices_to() fills that memory in.
            {
                let maxlen = cmp::max(xforms.len(), i2 - i1);
                self.stack.reserve(maxlen);
                let l = self.stack.len();
                unsafe { self.stack.set_len(l + maxlen) };
            }
            let (xfs1, xfs2) = self.stack.split_at_mut(i2);
            merge_slices_to(&xfs1[i1..i2], xforms, xfs2, |xf1, xf2| *xf1 * *xf2);
        }

        self.stack_indices.push(self.stack.len());
    }

    pub fn pop(&mut self) {
        assert!(self.stack_indices.len() > 2);

        let sl = self.stack.len();
        let sil = self.stack_indices.len();
        let i1 = self.stack_indices[sil - 2];
        let i2 = self.stack_indices[sil - 1];

        self.stack.truncate(sl - (i2 - i1));
        self.stack_indices.pop();
    }

    pub fn top<'a>(&'a self) -> &'a [Matrix4x4] {
        let sil = self.stack_indices.len();
        let i1 = self.stack_indices[sil - 2];
        let i2 = self.stack_indices[sil - 1];

        &self.stack[i1..i2]
    }
}
