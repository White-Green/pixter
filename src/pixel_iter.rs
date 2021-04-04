use partial_const::MayBeConst;
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};

use crate::physical_image::PhysicalImage;
use crate::{IntoPixelIterator, IntoSerializedPixelIterator};

pub struct PixIter<I: ParallelIterator + IndexedParallelIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> {
    width: W,
    height: H,
    iter: I,
}

impl<I: ParallelIterator + IndexedParallelIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> PixIter<I, W, H> {
    pub(crate) fn new(iter: I, width: W, height: H) -> Self {
        Self { width, height, iter }
    }
}

impl<I: ParallelIterator + IndexedParallelIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> PixIter<I, W, H> {
    pub fn width(&self) -> W {
        self.width
    }

    pub fn height(&self) -> H {
        self.height
    }

    pub fn into_inner(self) -> I {
        self.iter
    }

    pub fn collect_image(self) -> PhysicalImage<I::Item, W, H> {
        let PixIter { width, height, iter } = self;
        let mut data = Vec::with_capacity(width.value() * height.value());
        iter.collect_into_vec(&mut data);
        PhysicalImage::with_data(width, height, data)
    }
}

impl<I: ParallelIterator + IndexedParallelIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoPixelIterator for PixIter<I, W, H> {
    type Width = W;
    type Height = H;
    type Item = I::Item;
    type Iter = I;
    fn into_pix_iter(self) -> PixIter<Self::Iter, Self::Width, Self::Height> {
        self
    }
}

pub struct SerializePixIter<I: ExactSizeIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> {
    width: W,
    height: H,
    iter: I,
}

impl<I: ExactSizeIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> SerializePixIter<I, W, H> {
    pub(crate) fn new(iter: I, width: W, height: H) -> Self {
        Self { width, height, iter }
    }
}

impl<I: ExactSizeIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> SerializePixIter<I, W, H> {
    pub fn width(&self) -> W {
        self.width
    }

    pub fn height(&self) -> H {
        self.height
    }

    pub fn into_inner(self) -> I {
        self.iter
    }

    pub fn collect_image(self) -> PhysicalImage<I::Item, W, H> {
        let SerializePixIter { width, height, iter } = self;
        let data = iter.collect();
        PhysicalImage::with_data(width, height, data)
    }
}

impl<I: ExactSizeIterator, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoSerializedPixelIterator for SerializePixIter<I, W, H> {
    type Width = W;
    type Height = H;
    type Item = I::Item;
    type Iter = I;
    fn into_pix_iter_serialized(self) -> SerializePixIter<Self::Iter, Self::Width, Self::Height> {
        self
    }
}
