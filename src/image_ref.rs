use std::marker::PhantomData;

use partial_const::MayBeConst;

use crate::pixel_iter::{PixIter, SerializePixIter};
use crate::{IntoPixelIterator, IntoSerializedPixelIterator, ReadPixel, Rectangle, View, ViewMut, WritePixel};

mod iter;

pub struct ImageRef<'a, T, W: MayBeConst<usize> = usize, H: MayBeConst<usize> = usize> {
    base_width: usize,
    ptr: *const T,
    roi_x: usize,
    roi_y: usize,
    roi_width: W,
    roi_height: H,
    lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Send for ImageRef<'a, T, W, H> {}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Sync for ImageRef<'a, T, W, H> {}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRef<'a, T, W, H> {
    pub(crate) fn new(base_width: usize, ptr: *const T, roi_x: usize, roi_y: usize, roi_width: W, roi_height: H) -> Self {
        Self {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            lifetime: Default::default(),
        }
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ReadPixel for ImageRef<'a, T, W, H> {
    type Item = T;

    fn width(&self) -> usize {
        self.roi_width.value()
    }

    fn height(&self) -> usize {
        self.roi_height.value()
    }

    fn valid_rect(&self) -> Rectangle {
        Rectangle {
            x: 0,
            y: 0,
            w: self.roi_width.value(),
            h: self.roi_height.value(),
        }
    }

    unsafe fn get_unchecked(&self, x: usize, y: usize) -> &Self::Item {
        debug_assert!(self.is_valid(x, y), "Locate ({}, {}) is not valid in ImageRef::get_unchecked", x, y);
        &*self.ptr.add((self.roi_y + y) * self.base_width + self.roi_x + x)
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> View for ImageRef<'a, T, W, H> {
    fn view_is_valid<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> bool {
        x + w.value() <= self.roi_width.value() && y + h.value() <= self.roi_height.value()
    }

    unsafe fn view_unchecked<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> ImageRef<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in ImageRef::view_unchecked",
            x,
            y,
            w,
            h
        );
        ImageRef::new(self.base_width, self.ptr, x + self.roi_x, y + self.roi_y, w, h)
    }

    fn view_overhang<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhang<T, RW, RH> {
        let valid_x = x.clamp(0, self.roi_width.value() as isize) as usize;
        let valid_y = y.clamp(0, self.roi_height.value() as isize) as usize;
        let valid_width = (x + w.value() as isize).clamp(0, self.roi_width.value() as isize) as usize - valid_x;
        let valid_height = (y + h.value() as isize).clamp(0, self.roi_height.value() as isize) as usize - valid_y;
        ImageRefOverhang::new(
            unsafe { self.view_unchecked(valid_x, valid_y, valid_width, valid_height) },
            (-x).max(0) as usize,
            (-y).max(0) as usize,
            w,
            h,
        )
    }
}

impl<'a, T: 'a + Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRef<'a, T, W, H> {
    pub fn pix_iter(&self) -> PixIter<iter::Iter<'a, T>, W, H> {
        let &ImageRef {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        PixIter::new(
            iter::Iter::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRef<'a, T, W, H> {
    pub fn pix_iter_serialized(&self) -> SerializePixIter<iter::Iter<'a, T>, W, H> {
        let &ImageRef {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        SerializePixIter::new(
            iter::Iter::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

impl<'a, T: 'a + Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoPixelIterator for ImageRef<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = &'a T;
    type Iter = iter::Iter<'a, T>;

    fn into_pix_iter(self) -> PixIter<iter::Iter<'a, T>, W, H> {
        let ImageRef {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        PixIter::new(
            iter::Iter::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoSerializedPixelIterator for ImageRef<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = &'a T;
    type Iter = iter::Iter<'a, T>;

    fn into_pix_iter_serialized(self) -> SerializePixIter<iter::Iter<'a, T>, W, H> {
        let ImageRef {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        SerializePixIter::new(
            iter::Iter::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

pub struct ImageRefMut<'a, T, W: MayBeConst<usize> = usize, H: MayBeConst<usize> = usize> {
    base_width: usize,
    ptr: *mut T,
    roi_x: usize,
    roi_y: usize,
    roi_width: W,
    roi_height: H,
    lifetime: PhantomData<&'a mut ()>,
}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Send for ImageRefMut<'a, T, W, H> {}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Sync for ImageRefMut<'a, T, W, H> {}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefMut<'a, T, W, H> {
    pub(crate) fn new(base_width: usize, ptr: *mut T, roi_x: usize, roi_y: usize, roi_width: W, roi_height: H) -> Self {
        Self {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            lifetime: Default::default(),
        }
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ReadPixel for ImageRefMut<'a, T, W, H> {
    type Item = T;

    fn width(&self) -> usize {
        self.roi_width.value()
    }

    fn height(&self) -> usize {
        self.roi_height.value()
    }

    fn valid_rect(&self) -> Rectangle {
        Rectangle {
            x: 0,
            y: 0,
            w: self.roi_width.value(),
            h: self.roi_height.value(),
        }
    }

    unsafe fn get_unchecked(&self, x: usize, y: usize) -> &Self::Item {
        debug_assert!(self.is_valid(x, y), "Locate ({}, {}) is not valid in ImageRefMut::get_unchecked", x, y);
        &*self.ptr.add((self.roi_y + y) * self.base_width + self.roi_x + x)
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> WritePixel for ImageRefMut<'a, T, W, H> {
    unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize) -> &mut Self::Item {
        debug_assert!(self.is_valid(x, y), "Locate ({}, {}) is not valid in ImageRefMut::get_unchecked_mut", x, y);
        &mut *self.ptr.add((self.roi_y + y) * self.base_width + self.roi_x + x)
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> View for ImageRefMut<'a, T, W, H> {
    fn view_is_valid<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> bool {
        x + w.value() <= self.roi_width.value() && y + h.value() <= self.roi_height.value()
    }

    unsafe fn view_unchecked<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> ImageRef<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in ImageRefMut::view_unchecked",
            x,
            y,
            w,
            h
        );
        ImageRef::new(self.base_width, self.ptr, x + self.roi_x, y + self.roi_y, w, h)
    }

    fn view_overhang<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhang<T, RW, RH> {
        let valid_x = x.clamp(0, self.roi_width.value() as isize) as usize;
        let valid_y = y.clamp(0, self.roi_height.value() as isize) as usize;
        let valid_width = (x + w.value() as isize).clamp(0, self.roi_width.value() as isize) as usize - valid_x;
        let valid_height = (y + h.value() as isize).clamp(0, self.roi_height.value() as isize) as usize - valid_y;
        ImageRefOverhang::new(
            unsafe { self.view_unchecked(valid_x, valid_y, valid_width, valid_height) },
            (-x).max(0) as usize,
            (-y).max(0) as usize,
            w,
            h,
        )
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ViewMut for ImageRefMut<'a, T, W, H> {
    unsafe fn view_unchecked_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: usize, y: usize, w: RW, h: RH) -> ImageRefMut<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in ImageRefMut::view_unchecked_mut",
            x,
            y,
            w,
            h
        );
        ImageRefMut::new(self.base_width, self.ptr, x + self.roi_x, y + self.roi_y, w, h)
    }

    fn view_overhang_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhangMut<T, RW, RH> {
        let valid_x = x.clamp(0, self.roi_width.value() as isize) as usize;
        let valid_y = y.clamp(0, self.roi_height.value() as isize) as usize;
        let valid_width = (x + w.value() as isize).clamp(0, self.roi_width.value() as isize) as usize - valid_x;
        let valid_height = (y + h.value() as isize).clamp(0, self.roi_height.value() as isize) as usize - valid_y;
        ImageRefOverhangMut::new(
            unsafe { self.view_unchecked_mut(valid_x, valid_y, valid_width, valid_height) },
            (-x).max(0) as usize,
            (-y).max(0) as usize,
            w,
            h,
        )
    }
}

impl<'a, T: 'a + Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefMut<'a, T, W, H> {
    pub fn pix_iter(&self) -> PixIter<iter::Iter<'a, T>, W, H> {
        let &ImageRefMut {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        PixIter::new(
            iter::Iter::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

impl<'a, T: 'a + Send, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefMut<'a, T, W, H> {
    pub fn pix_iter_mut(&mut self) -> PixIter<iter::IterMut<'a, T>, W, H> {
        let &mut ImageRefMut {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        PixIter::new(
            iter::IterMut::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefMut<'a, T, W, H> {
    pub fn pix_iter_serialized(&self) -> SerializePixIter<iter::Iter<'a, T>, W, H> {
        let &ImageRefMut {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        SerializePixIter::new(
            iter::Iter::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }

    pub fn pix_iter_serialized_mut(&mut self) -> SerializePixIter<iter::IterMut<'a, T>, W, H> {
        let &mut ImageRefMut {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        SerializePixIter::new(
            iter::IterMut::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

impl<'a, T: 'a + Send, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoPixelIterator for ImageRefMut<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = &'a mut T;
    type Iter = iter::IterMut<'a, T>;

    fn into_pix_iter(self) -> PixIter<iter::IterMut<'a, T>, W, H> {
        let ImageRefMut {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        PixIter::new(
            iter::IterMut::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoSerializedPixelIterator for ImageRefMut<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = &'a mut T;
    type Iter = iter::IterMut<'a, T>;

    fn into_pix_iter_serialized(self) -> SerializePixIter<iter::IterMut<'a, T>, W, H> {
        let ImageRefMut {
            base_width,
            ptr,
            roi_x,
            roi_y,
            roi_width,
            roi_height,
            ..
        } = self;
        let offset = roi_y * roi_width.value();
        SerializePixIter::new(
            iter::IterMut::new(ptr, base_width, roi_x, roi_width.value(), offset..offset + roi_width.value() * roi_height.value()),
            roi_width,
            roi_height,
        )
    }
}

pub struct ImageRefOverhang<'a, T, W: MayBeConst<usize> = usize, H: MayBeConst<usize> = usize> {
    valid_ref: ImageRef<'a, T, usize, usize>,
    valid_offset_x: usize,
    valid_offset_y: usize,
    width: W,
    height: H,
}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Send for ImageRefOverhang<'a, T, W, H> {}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Sync for ImageRefOverhang<'a, T, W, H> {}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefOverhang<'a, T, W, H> {
    pub(crate) fn new(valid_ref: ImageRef<'a, T, usize, usize>, valid_offset_x: usize, valid_offset_y: usize, width: W, height: H) -> Self {
        Self {
            valid_ref,
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        }
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ReadPixel for ImageRefOverhang<'a, T, W, H> {
    type Item = T;

    fn width(&self) -> usize {
        self.width.value()
    }

    fn height(&self) -> usize {
        self.height.value()
    }

    fn valid_rect(&self) -> Rectangle {
        Rectangle {
            x: self.valid_offset_x,
            y: self.valid_offset_y,
            w: self.valid_ref.roi_width,
            h: self.valid_ref.roi_height,
        }
    }

    unsafe fn get_unchecked(&self, x: usize, y: usize) -> &Self::Item {
        debug_assert!(self.is_valid(x, y), "Locate ({}, {}) is not valid in ImageRefOverhang::get_unchecked", x, y);
        self.valid_ref.get_unchecked(x - self.valid_offset_x, y - self.valid_offset_y)
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> View for ImageRefOverhang<'a, T, W, H> {
    fn view_is_valid<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> bool {
        x.checked_sub(self.valid_offset_x).map(|x| x + w.value() <= self.valid_ref.roi_width).unwrap_or(false)
            && y.checked_sub(self.valid_offset_y).map(|y| y + h.value() <= self.valid_ref.roi_height).unwrap_or(false)
    }

    unsafe fn view_unchecked<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> ImageRef<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in ImageRefOverhang::view_unchecked",
            x,
            y,
            w,
            h
        );
        self.valid_ref.view_unchecked(x - self.valid_offset_x, y - self.valid_offset_y, w, h)
    }

    fn view_overhang<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhang<T, RW, RH> {
        self.valid_ref.view_overhang(x - self.valid_offset_x as isize, y - self.valid_offset_y as isize, w, h)
    }
}

impl<'a, T: 'a + Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefOverhang<'a, T, W, H> {
    pub fn pix_iter(&self) -> PixIter<iter::IterOverhang<iter::Iter<'a, T>>, W, H> {
        let &ImageRefOverhang {
            valid_ref:
                ImageRef {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        PixIter::new(
            iter::IterOverhang::new(
                iter::Iter::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefOverhang<'a, T, W, H> {
    pub fn pix_iter_serialized(&self) -> SerializePixIter<iter::IterOverhang<iter::Iter<'a, T>>, W, H> {
        let &ImageRefOverhang {
            valid_ref:
                ImageRef {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        SerializePixIter::new(
            iter::IterOverhang::new(
                iter::Iter::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

impl<'a, T: 'a + Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoPixelIterator for ImageRefOverhang<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = Option<&'a T>;
    type Iter = iter::IterOverhang<iter::Iter<'a, T>>;

    fn into_pix_iter(self) -> PixIter<iter::IterOverhang<iter::Iter<'a, T>>, W, H> {
        let ImageRefOverhang {
            valid_ref:
                ImageRef {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        PixIter::new(
            iter::IterOverhang::new(
                iter::Iter::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoSerializedPixelIterator for ImageRefOverhang<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = Option<&'a T>;
    type Iter = iter::IterOverhang<iter::Iter<'a, T>>;

    fn into_pix_iter_serialized(self) -> SerializePixIter<iter::IterOverhang<iter::Iter<'a, T>>, W, H> {
        let ImageRefOverhang {
            valid_ref:
                ImageRef {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        SerializePixIter::new(
            iter::IterOverhang::new(
                iter::Iter::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

pub struct ImageRefOverhangMut<'a, T, W: MayBeConst<usize> = usize, H: MayBeConst<usize> = usize> {
    valid_ref: ImageRefMut<'a, T, usize, usize>,
    valid_offset_x: usize,
    valid_offset_y: usize,
    width: W,
    height: H,
}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Send for ImageRefOverhangMut<'a, T, W, H> {}

unsafe impl<'a, T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> Sync for ImageRefOverhangMut<'a, T, W, H> {}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefOverhangMut<'a, T, W, H> {
    pub(crate) fn new(valid_ref: ImageRefMut<'a, T, usize, usize>, valid_offset_x: usize, valid_offset_y: usize, width: W, height: H) -> Self {
        Self {
            valid_ref,
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        }
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ReadPixel for ImageRefOverhangMut<'a, T, W, H> {
    type Item = T;

    fn width(&self) -> usize {
        self.width.value()
    }

    fn height(&self) -> usize {
        self.height.value()
    }

    fn valid_rect(&self) -> Rectangle {
        Rectangle {
            x: self.valid_offset_x,
            y: self.valid_offset_y,
            w: self.valid_ref.roi_width,
            h: self.valid_ref.roi_height,
        }
    }

    unsafe fn get_unchecked(&self, x: usize, y: usize) -> &Self::Item {
        debug_assert!(self.is_valid(x, y), "Locate ({}, {}) is not valid in ImageRefOverhangMut::get_unchecked", x, y);
        self.valid_ref.get_unchecked(x - self.valid_offset_x, y - self.valid_offset_y)
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> WritePixel for ImageRefOverhangMut<'a, T, W, H> {
    unsafe fn get_unchecked_mut(&mut self, x: usize, y: usize) -> &mut Self::Item {
        debug_assert!(self.is_valid(x, y), "Locate ({}, {}) is not valid in ImageRefOverhangMut::get_unchecked_mut", x, y);
        self.valid_ref.get_unchecked_mut(x - self.valid_offset_x, y - self.valid_offset_y)
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> View for ImageRefOverhangMut<'a, T, W, H> {
    fn view_is_valid<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> bool {
        x.checked_sub(self.valid_offset_x).map(|x| x + w.value() <= self.valid_ref.roi_width).unwrap_or(false)
            && y.checked_sub(self.valid_offset_y).map(|y| y + h.value() <= self.valid_ref.roi_height).unwrap_or(false)
    }

    unsafe fn view_unchecked<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> ImageRef<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in ImageRefOverhang::view_unchecked_mut",
            x,
            y,
            w,
            h
        );
        self.valid_ref.view_unchecked(x - self.valid_offset_x, y - self.valid_offset_y, w, h)
    }

    fn view_overhang<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhang<T, RW, RH> {
        self.valid_ref.view_overhang(x - self.valid_offset_x as isize, y - self.valid_offset_y as isize, w, h)
    }
}

impl<'a, T, W: MayBeConst<usize>, H: MayBeConst<usize>> ViewMut for ImageRefOverhangMut<'a, T, W, H> {
    unsafe fn view_unchecked_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: usize, y: usize, w: RW, h: RH) -> ImageRefMut<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in ImageRefOverhangMut::view_unchecked_mut",
            x,
            y,
            w,
            h
        );
        self.valid_ref.view_unchecked_mut(x - self.valid_offset_x, y - self.valid_offset_y, w, h)
    }

    fn view_overhang_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhangMut<T, RW, RH> {
        self.valid_ref.view_overhang_mut(x - self.valid_offset_x as isize, y - self.valid_offset_y as isize, w, h)
    }
}

impl<'a, T: 'a + Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefOverhangMut<'a, T, W, H> {
    pub fn pix_iter(&self) -> PixIter<iter::IterOverhang<iter::Iter<'a, T>>, W, H> {
        let &ImageRefOverhangMut {
            valid_ref:
                ImageRefMut {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        PixIter::new(
            iter::IterOverhang::new(
                iter::Iter::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

impl<'a, T: 'a + Send, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefOverhangMut<'a, T, W, H> {
    pub fn pix_iter_mut(&mut self) -> PixIter<iter::IterOverhang<iter::IterMut<'a, T>>, W, H> {
        let &mut ImageRefOverhangMut {
            valid_ref:
                ImageRefMut {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        PixIter::new(
            iter::IterOverhang::new(
                iter::IterMut::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> ImageRefOverhangMut<'a, T, W, H> {
    pub fn pix_iter_serialized(&self) -> SerializePixIter<iter::IterOverhang<iter::Iter<'a, T>>, W, H> {
        let &ImageRefOverhangMut {
            valid_ref:
                ImageRefMut {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        SerializePixIter::new(
            iter::IterOverhang::new(
                iter::Iter::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }

    pub fn pix_iter_serialized_mut(&mut self) -> SerializePixIter<iter::IterOverhang<iter::IterMut<'a, T>>, W, H> {
        let &mut ImageRefOverhangMut {
            valid_ref:
                ImageRefMut {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        SerializePixIter::new(
            iter::IterOverhang::new(
                iter::IterMut::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

impl<'a, T: 'a + Send, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoPixelIterator for ImageRefOverhangMut<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = Option<&'a mut T>;
    type Iter = iter::IterOverhang<iter::IterMut<'a, T>>;

    fn into_pix_iter(self) -> PixIter<iter::IterOverhang<iter::IterMut<'a, T>>, W, H> {
        let ImageRefOverhangMut {
            valid_ref:
                ImageRefMut {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        PixIter::new(
            iter::IterOverhang::new(
                iter::IterMut::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

impl<'a, T: 'a, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoSerializedPixelIterator for ImageRefOverhangMut<'a, T, W, H> {
    type Width = W;
    type Height = H;
    type Item = Option<&'a mut T>;
    type Iter = iter::IterOverhang<iter::IterMut<'a, T>>;

    fn into_pix_iter_serialized(self) -> SerializePixIter<iter::IterOverhang<iter::IterMut<'a, T>>, W, H> {
        let ImageRefOverhangMut {
            valid_ref:
                ImageRefMut {
                    base_width,
                    ptr,
                    roi_x,
                    roi_y,
                    roi_width,
                    roi_height,
                    ..
                },
            valid_offset_x,
            valid_offset_y,
            width,
            height,
        } = self;
        let offset = roi_y * roi_width;

        SerializePixIter::new(
            iter::IterOverhang::new(
                iter::IterMut::new(ptr, base_width, roi_x, roi_width, offset..offset + roi_width * roi_height),
                roi_width,
                roi_height,
                valid_offset_x,
                valid_offset_y,
                width.value(),
                height.value(),
            ),
            width,
            height,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::physical_image::PhysicalImage;
    use crate::{IntoPixelIterator, IntoSerializedPixelIterator, ReadPixel, View, ViewMut, WritePixel};

    #[test]
    fn view() {
        const WIDTH: usize = 50;
        const HEIGHT: usize = 50;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = WIDTH * y + x;
            }
        }
        let image_ref = image.view(10usize, 10usize, 30usize, 30usize).unwrap();
        for y in 0..30 {
            for x in 0..30 {
                assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 10) + x + 10)));
            }
        }
        assert!(image_ref.view(20, 20, 10, 11).is_none());
        assert!(image_ref.view(20, 20, 11, 10).is_none());
        assert!(image_ref.view(20, 21, 10, 10).is_none());
        assert!(image_ref.view(21, 20, 10, 10).is_none());
        {
            let image_ref = image_ref.view(10usize, 10usize, 10usize, 10usize).unwrap();
            for y in 0..10 {
                for x in 0..10 {
                    assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 20) + x + 20)));
                }
            }
        }
        {
            let image_ref = image_ref.view_overhang(-10, -10, 20, 20);
            for y in 0..20 {
                for x in 0..20 {
                    if x < 10 || y < 10 {
                        assert_eq!(image_ref.get(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y) + x)));
                    }
                }
            }
        }
        {
            let image_ref = image_ref.view_overhang(20, -10, 20, 20);
            for y in 0..20 {
                for x in 0..20 {
                    if x >= 10 || y < 10 {
                        assert_eq!(image_ref.get(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y) + x + 30)));
                    }
                }
            }
        }
        {
            let image_ref = image_ref.view_overhang(-10, 20, 20, 20);
            for y in 0..20 {
                for x in 0..20 {
                    if x < 10 || y >= 10 {
                        assert_eq!(image_ref.get(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 30) + x)));
                    }
                }
            }
        }
        {
            let image_ref = image_ref.view_overhang(20, 20, 20, 20);
            for y in 0..20 {
                for x in 0..20 {
                    if x >= 10 || y >= 10 {
                        assert_eq!(image_ref.get(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 30) + x + 30)));
                    }
                }
            }
        }
        let mut image_ref = image.view_mut(10, 10, 30, 30).unwrap();
        assert!(image_ref.view(20, 20, 10, 11).is_none());
        assert!(image_ref.view(20, 20, 11, 10).is_none());
        assert!(image_ref.view(20, 21, 10, 10).is_none());
        assert!(image_ref.view(21, 20, 10, 10).is_none());
        assert!(image_ref.view_mut(20, 20, 10, 11).is_none());
        assert!(image_ref.view_mut(20, 20, 11, 10).is_none());
        assert!(image_ref.view_mut(20, 21, 10, 10).is_none());
        assert!(image_ref.view_mut(21, 20, 10, 10).is_none());
        {
            let image_ref = image_ref.view(10usize, 10usize, 10usize, 10usize).unwrap();
            for y in 0..10 {
                for x in 0..10 {
                    assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 20) + x + 20)));
                }
            }
        }
        {
            let mut image_ref = image_ref.view_mut(10usize, 10usize, 10usize, 10usize).unwrap();
            for y in 0..10 {
                for x in 0..10 {
                    assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 20) + x + 20)));
                    assert_eq!(image_ref.get_mut(x, y), Some(&mut (WIDTH * (y + 20) + x + 20)));
                }
            }
        }
        {
            let image_ref = image_ref.view_overhang(-5, -5, 10, 10);
            for y in 0..10 {
                for x in 0..10 {
                    if x < 5 || y < 5 {
                        assert_eq!(image_ref.get(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 5) + x + 5)));
                    }
                }
            }
        }
        {
            let mut image_ref = image_ref.view_overhang_mut(-5, -5, 10, 10);
            for y in 0..10 {
                for x in 0..10 {
                    if x < 5 || y < 5 {
                        assert_eq!(image_ref.get(x, y), None);
                        assert_eq!(image_ref.get_mut(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&(WIDTH * (y + 5) + x + 5)));
                        assert_eq!(image_ref.get_mut(x, y), Some(&mut (WIDTH * (y + 5) + x + 5)));
                    }
                }
            }
        }
    }

    #[test]
    fn overhang() {
        const WIDTH: usize = 50;
        const HEIGHT: usize = 50;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = WIDTH * y + x;
            }
        }
        let image_ref = image.view_overhang(10, 10, 30, 30);
        assert!(image_ref.view(20, 20, 10, 11).is_none());
        assert!(image_ref.view(20, 20, 11, 10).is_none());
        assert!(image_ref.view(20, 21, 10, 10).is_none());
        assert!(image_ref.view(21, 20, 10, 10).is_none());
        {
            let image_ref = image_ref.view(20, 20, 10, 10).unwrap();
            for y in 0..10 {
                for x in 0..10 {
                    assert_eq!(image_ref.get(x, y), Some(&((y + 30) * WIDTH + x + 30)));
                }
            }
        }
        {
            let image_ref = image.view_overhang(-10, -10, 20, 20);
            let image_ref = image_ref.view_overhang(5, 5, 10, 10);
            for y in 0..10 {
                for x in 0..10 {
                    if x < 5 || y < 5 {
                        assert_eq!(image_ref.get(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&((y - 5) * WIDTH + x - 5)));
                    }
                }
            }
        }
        {
            let image_ref = image.view_overhang_mut(-10, -10, 20, 20);
            let image_ref = image_ref.view_overhang(5, 5, 10, 10);
            for y in 0..10 {
                for x in 0..10 {
                    if x < 5 || y < 5 {
                        assert_eq!(image_ref.get(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&((y - 5) * WIDTH + x - 5)));
                    }
                }
            }
        }
        {
            let mut image_ref = image.view_overhang_mut(-10, -10, 20, 20);
            let mut image_ref = image_ref.view_overhang_mut(5, 5, 10, 10);
            for y in 0..10 {
                for x in 0..10 {
                    if x < 5 || y < 5 {
                        assert_eq!(image_ref.get(x, y), None);
                        assert_eq!(image_ref.get_mut(x, y), None);
                    } else {
                        assert_eq!(image_ref.get(x, y), Some(&((y - 5) * WIDTH + x - 5)));
                        assert_eq!(image_ref.get_mut(x, y), Some(&mut ((y - 5) * WIDTH + x - 5)));
                    }
                }
            }
        }
        let image_ref = image.view_overhang(-10, 10, 30, 30);
        assert!(image_ref.view(9, 0, 10, 10).is_none());
        assert!(image_ref.view(10, 0, 10, 10).is_some());
        let image_ref = image.view_overhang(10, -10, 30, 30);
        assert!(image_ref.view(0, 9, 10, 10).is_none());
        assert!(image_ref.view(0, 10, 10, 10).is_some());
        let image_ref = image.view_overhang(40, 10, 30, 30);
        assert!(image_ref.view(1, 0, 10, 10).is_none());
        assert!(image_ref.view(0, 0, 10, 10).is_some());
        let image_ref = image.view_overhang(10, 40, 30, 30);
        assert!(image_ref.view(0, 1, 10, 10).is_none());
        assert!(image_ref.view(0, 0, 10, 10).is_some());
        let mut image_ref = image.view_overhang_mut(-10, 10, 30, 30);
        assert!(image_ref.view(9, 0, 10, 10).is_none());
        assert!(image_ref.view(10, 0, 10, 10).is_some());
        assert!(image_ref.view_mut(9, 0, 10, 10).is_none());
        assert!(image_ref.view_mut(10, 0, 10, 10).is_some());
        let mut image_ref = image.view_overhang_mut(10, -10, 30, 30);
        assert!(image_ref.view(0, 9, 10, 10).is_none());
        assert!(image_ref.view(0, 10, 10, 10).is_some());
        assert!(image_ref.view_mut(0, 9, 10, 10).is_none());
        assert!(image_ref.view_mut(0, 10, 10, 10).is_some());
        let mut image_ref = image.view_overhang_mut(40, 10, 30, 30);
        assert!(image_ref.view(1, 0, 10, 10).is_none());
        assert!(image_ref.view(0, 0, 10, 10).is_some());
        assert!(image_ref.view_mut(1, 0, 10, 10).is_none());
        assert!(image_ref.view_mut(0, 0, 10, 10).is_some());
        let mut image_ref = image.view_overhang_mut(10, 40, 30, 30);
        assert!(image_ref.view(0, 1, 10, 10).is_none());
        assert!(image_ref.view(0, 0, 10, 10).is_some());
        assert!(image_ref.view_mut(0, 1, 10, 10).is_none());
        assert!(image_ref.view_mut(0, 0, 10, 10).is_some());
    }

    #[test]
    fn iterator() {
        const WIDTH: usize = 50;
        const HEIGHT: usize = 50;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = WIDTH * y + x;
            }
        }
        let image_data = image.data.clone();

        let iter = image.view(0usize, 0usize, WIDTH, HEIGHT).unwrap().pix_iter();
        let iterated = iter.collect_image();
        assert_eq!(image_data.iter().collect::<Vec<_>>(), iterated.data);

        let iter = image.view(10usize, 10usize, 30usize, 30usize).unwrap().pix_iter();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40).into_iter().map(|i| i * WIDTH + 10..i * WIDTH + 40).flatten().collect::<Vec<_>>().iter().collect::<Vec<_>>()
        );

        let iter = image.view(0usize, 0usize, WIDTH, HEIGHT).unwrap().pix_iter_serialized();
        let iterated = iter.collect_image();
        assert_eq!(image_data.iter().collect::<Vec<_>>(), iterated.data);

        let iter = image.view(10usize, 10usize, 30usize, 30usize).unwrap().pix_iter_serialized();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40).into_iter().map(|i| i * WIDTH + 10..i * WIDTH + 40).flatten().collect::<Vec<_>>().iter().collect::<Vec<_>>()
        );

        let iter = image.view_mut(0usize, 0usize, WIDTH, HEIGHT).unwrap().pix_iter();
        let iterated = iter.collect_image();
        assert_eq!(image_data.clone().iter_mut().collect::<Vec<_>>(), iterated.data);

        let iter = image.view_mut(10usize, 10usize, 30usize, 30usize).unwrap().pix_iter();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40)
                .into_iter()
                .map(|i| i * WIDTH + 10..i * WIDTH + 40)
                .flatten()
                .collect::<Vec<_>>()
                .iter_mut()
                .collect::<Vec<_>>()
        );

        let iter = image.view_mut(0usize, 0usize, WIDTH, HEIGHT).unwrap().pix_iter_mut();
        let iterated = iter.collect_image();
        assert_eq!(image_data.clone().iter_mut().collect::<Vec<_>>(), iterated.data);

        let iter = image.view_mut(10usize, 10usize, 30usize, 30usize).unwrap().pix_iter_mut();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40)
                .into_iter()
                .map(|i| i * WIDTH + 10..i * WIDTH + 40)
                .flatten()
                .collect::<Vec<_>>()
                .iter_mut()
                .collect::<Vec<_>>()
        );

        let iter = image.view_mut(0usize, 0usize, WIDTH, HEIGHT).unwrap().pix_iter_serialized();
        let iterated = iter.collect_image();
        assert_eq!(image_data.clone().iter_mut().collect::<Vec<_>>(), iterated.data);

        let iter = image.view_mut(10usize, 10usize, 30usize, 30usize).unwrap().pix_iter_serialized();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40)
                .into_iter()
                .map(|i| i * WIDTH + 10..i * WIDTH + 40)
                .flatten()
                .collect::<Vec<_>>()
                .iter_mut()
                .collect::<Vec<_>>()
        );

        let iter = image.view_mut(0usize, 0usize, WIDTH, HEIGHT).unwrap().pix_iter_serialized_mut();
        let iterated = iter.collect_image();
        assert_eq!(image_data.clone().iter_mut().collect::<Vec<_>>(), iterated.data);

        let iter = image.view_mut(10usize, 10usize, 30usize, 30usize).unwrap().pix_iter_serialized_mut();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40)
                .into_iter()
                .map(|i| i * WIDTH + 10..i * WIDTH + 40)
                .flatten()
                .collect::<Vec<_>>()
                .iter_mut()
                .collect::<Vec<_>>()
        );

        let iter = image.view(0usize, 0usize, WIDTH, HEIGHT).unwrap().into_pix_iter();
        let iterated = iter.collect_image();
        assert_eq!(image_data.iter().collect::<Vec<_>>(), iterated.data);

        let iter = image.view(10usize, 10usize, 30usize, 30usize).unwrap().into_pix_iter();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40).into_iter().map(|i| i * WIDTH + 10..i * WIDTH + 40).flatten().collect::<Vec<_>>().iter().collect::<Vec<_>>()
        );

        let iter = image.view(0usize, 0usize, WIDTH, HEIGHT).unwrap().into_pix_iter_serialized();
        let iterated = iter.collect_image();
        assert_eq!(image_data.iter().collect::<Vec<_>>(), iterated.data);

        let iter = image.view(10usize, 10usize, 30usize, 30usize).unwrap().into_pix_iter_serialized();
        let iterated = iter.collect_image();
        assert_eq!(
            iterated.data,
            (10..40).into_iter().map(|i| i * WIDTH + 10..i * WIDTH + 40).flatten().collect::<Vec<_>>().iter().collect::<Vec<_>>()
        );
    }
}
