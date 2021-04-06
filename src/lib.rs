#![warn(missing_docs)]
#![warn(missing_crate_level_docs)]

//! A crate for image processing by processing for each pixels.

use partial_const::MayBeConst;
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};

use crate::image_ref::{ImageRef, ImageRefMut, ImageRefOverhang, ImageRefOverhangMut};
use crate::pixel_iter::{PixIter, SerializePixIter};

pub mod image_ref;
pub mod physical_image;
pub mod pixel_iter;

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rectangle {
    pub fn contains(&self, x: usize, y: usize) -> bool {
        self.x <= x && x < self.x + self.w && self.y <= y && y < self.y + self.h
    }
}

/// A trait for reading value of specified pixel.
pub trait ReadPixel {
    /// A type of item in each pixel.
    type Item;
    /// Get width of image.
    fn width(&self) -> usize;
    /// Get height of image.
    fn height(&self) -> usize;
    /// Get valid area in this image.
    fn valid_rect(&self) -> Rectangle;
    /// Check weather that pixel (x, y) have valid value or not.
    /// You can use this instead of valid_rect().contains(x, y);
    fn is_valid(&self, x: usize, y: usize) -> bool {
        self.valid_rect().contains(x, y)
    }
    /// Get value reference of pixel (x, y).
    /// If and only if is_valid(x, y) == false, this function returns None.
    fn get(&self, x: usize, y: usize) -> Option<&Self::Item> {
        if self.is_valid(x, y) {
            Some(unsafe { self.get_unchecked(x, y) })
        } else {
            None
        }
    }
    /// Get value reference of pixel (x, y) without checking.
    /// # Safety
    /// Point (x, y) should be included in valid_rect.
    unsafe fn get_unchecked(&self, x: usize, y: usize) -> &Self::Item;
}

/// A trait for editing value of specified pixel.
pub trait WritePixel: ReadPixel {
    /// Get mutable value reference of pixel (x, y).
    /// If and only if is_valid(x, y) == false, this function returns None.
    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Self::Item> {
        if self.is_valid(x, y) {
            Some(unsafe { self.get_unchecked_mut(x, y) })
        } else {
            None
        }
    }
    /// Get mutable value reference of pixel (x, y) without checking.
    /// # Safety
    /// Point (x, y) should be included in valid_rect.
    unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize) -> &mut Self::Item;
}

/// A trait for getting area reference of image.
pub trait View: ReadPixel {
    /// Check weather that specified rectangle is valid or not.
    fn view_is_valid<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> bool;
    /// Get area reference of image.
    /// If and only if view_is_valid(x, y, w, h) == false, this function returns None.
    fn view<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> Option<ImageRef<Self::Item, RW, RH>> {
        if self.view_is_valid(x, y, w, h) {
            Some(unsafe { self.view_unchecked(x, y, w, h) })
        } else {
            None
        }
    }
    /// Get area reference of image without checking.
    /// # Safety
    /// Rectangle {x, y, w, h} should be valid.
    unsafe fn view_unchecked<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> ImageRef<Self::Item, RW, RH>;
    fn view_overhang<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhang<Self::Item, RW, RH>;
}

/// A trait for getting mutable area reference of image.
pub trait ViewMut: View + WritePixel {
    /// Get mutable area reference of image.
    /// If and only if view_is_valid(x, y, w, h) == false, this function returns None.
    fn view_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: usize, y: usize, w: RW, h: RH) -> Option<ImageRefMut<Self::Item, RW, RH>> {
        if self.view_is_valid(x, y, w, h) {
            Some(unsafe { self.view_unchecked_mut(x, y, w, h) })
        } else {
            None
        }
    }
    /// Get mutable area reference of image without checking.
    /// # Safety
    /// Rectangle {x, y, w, h} should be valid.
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
