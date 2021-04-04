#![warn(missing_docs)]
#![warn(missing_crate_level_docs)]

use partial_const::MayBeConst;
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};

use crate::image_ref::{ImageRef, ImageRefMut, ImageRefOverhang, ImageRefOverhangMut};
use crate::pixel_iter::{PixIter, SerializePixIter};

pub mod image_ref;
pub mod physical_image;
pub mod pixel_iter;

pub trait ReadPixel {
    type Item;
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn is_valid(&self, x: usize, y: usize) -> bool;
    fn get(&self, x: usize, y: usize) -> Option<&Self::Item> {
        if self.is_valid(x, y) {
            Some(unsafe { self.get_unchecked(x, y) })
        } else {
            None
        }
    }
    unsafe fn get_unchecked(&self, x: usize, y: usize) -> &Self::Item;
}

pub trait WritePixel: ReadPixel {
    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Self::Item> {
        if self.is_valid(x, y) {
            Some(unsafe { self.get_unchecked_mut(x, y) })
        } else {
            None
        }
    }
    unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize) -> &mut Self::Item;
}

pub trait View: ReadPixel {
    fn view_is_valid<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> bool;
    fn view<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> Option<ImageRef<Self::Item, RW, RH>> {
        if self.view_is_valid(x, y, w, h) {
            Some(unsafe { self.view_unchecked(x, y, w, h) })
        } else {
            None
        }
    }
    unsafe fn view_unchecked<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> ImageRef<Self::Item, RW, RH>;
    fn view_overhang<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhang<Self::Item, RW, RH>;
}

pub trait ViewMut: View + WritePixel {
    fn view_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: usize, y: usize, w: RW, h: RH) -> Option<ImageRefMut<Self::Item, RW, RH>> {
        if self.view_is_valid(x, y, w, h) {
            Some(unsafe { self.view_unchecked_mut(x, y, w, h) })
        } else {
            None
        }
    }
    unsafe fn view_unchecked_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: usize, y: usize, w: RW, h: RH) -> ImageRefMut<Self::Item, RW, RH>;
    fn view_overhang_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhangMut<Self::Item, RW, RH>;
}

pub trait IntoPixelIterator {
    type Width: MayBeConst<usize>;
    type Height: MayBeConst<usize>;
    type Item;
    type Iter: ParallelIterator<Item = Self::Item> + IndexedParallelIterator;
    fn into_pix_iter(self) -> PixIter<Self::Iter, Self::Width, Self::Height>;
}

pub trait IntoSerializedPixelIterator {
    type Width: MayBeConst<usize>;
    type Height: MayBeConst<usize>;
    type Item;
    type Iter: ExactSizeIterator<Item = Self::Item>;
    fn into_pix_iter_serialized(self) -> SerializePixIter<Self::Iter, Self::Width, Self::Height>;
}
