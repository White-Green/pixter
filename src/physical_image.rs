use std::error::Error;
use std::path::Path;

use image::buffer::ConvertBuffer;
use image::{Bgr, Bgra, DynamicImage, EncodableLayout, ImageBuffer, ImageResult, Luma, LumaA, Pixel, Rgb, Rgba};
use partial_const::MayBeConst;
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::image_ref::{ImageRef, ImageRefMut, ImageRefOverhang, ImageRefOverhangMut};
use crate::pixel_iter::{PixIter, SerializePixIter};
use crate::{IntoPixelIterator, IntoSerializedPixelIterator, ReadPixel, View, ViewMut, WritePixel};

#[derive(Debug)]
pub struct PhysicalImage<T, W: MayBeConst<usize> = usize, H: MayBeConst<usize> = usize> {
    width: W,
    height: H,
    pub(crate) data: Vec<T>,
}

impl<T, W: MayBeConst<usize>, H: MayBeConst<usize>> PhysicalImage<T, W, H> {
    pub fn new(width: W, height: H) -> Self
    where
        T: Default,
        for<'a> &'a mut [T]: IntoParallelIterator<Item = &'a mut T>,
    {
        unsafe {
            let mut image = Self::new_uninit(width, height);
            image.data.par_iter_mut().for_each(|value| {
                let value: *mut T = value;
                value.write(Default::default());
            });
            image
        }
    }

    pub fn with_default(width: W, height: H, default: T) -> Self
    where
        T: Clone + Sync,
        for<'a> &'a mut [T]: IntoParallelIterator<Item = &'a mut T>,
    {
        unsafe {
            let mut image = Self::new_uninit(width, height);
            image.data.par_iter_mut().for_each(|value| {
                let value: *mut T = value;
                value.write(default.clone());
            });
            image
        }
    }

    pub unsafe fn new_uninit(width: W, height: H) -> Self {
        let mut data = Vec::<T>::with_capacity(width.value() * height.value());
        data.set_len(width.value() * height.value());
        Self { width, height, data }
    }

    pub(crate) fn with_data(width: W, height: H, data: Vec<T>) -> Self {
        debug_assert_eq!(data.len(), width.value() * height.value());
        Self { width, height, data }
    }
}

impl<P> PhysicalImage<P, usize, usize>
where
    Self: From<DynamicImage>,
{
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        image::io::Reader::open(path)
            .map_err(|e| Box::new(e) as Box<dyn Error>)?
            .decode()
            .map(Into::into)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}

impl<P: 'static + Pixel, W: MayBeConst<usize>, H: MayBeConst<usize>> PhysicalImage<P, W, H>
where
    Self: Into<ImageBuffer<P, Vec<P::Subpixel>>>,
    [P::Subpixel]: EncodableLayout,
{
    pub fn save(self, path: impl AsRef<Path>) -> ImageResult<()> {
        let image_buffer = self.into();
        image_buffer.save(path)
    }
}

impl<P: 'static + Pixel + Send> From<ImageBuffer<P, Vec<P::Subpixel>>> for PhysicalImage<P, usize, usize>
where
    Vec<P::Subpixel>: IntoParallelIterator<Item = P::Subpixel>,
    <Vec<P::Subpixel> as IntoParallelIterator>::Iter: IndexedParallelIterator,
{
    fn from(image: ImageBuffer<P, Vec<<P as Pixel>::Subpixel>>) -> Self {
        let width = image.width() as usize;
        let height = image.height() as usize;
        let mut data = Vec::with_capacity(width * height);
        image
            .into_vec()
            .into_par_iter()
            .chunks(P::CHANNEL_COUNT as usize)
            .map(|v| *P::from_slice(&v))
            .collect_into_vec(&mut data);
        Self { width, height, data }
    }
}

impl<P: 'static + Pixel, W: MayBeConst<usize>, H: MayBeConst<usize>> From<PhysicalImage<P, W, H>> for ImageBuffer<P, Vec<P::Subpixel>>
where
    Vec<P>: IntoParallelIterator<Item = P>,
    <Vec<P> as IntoParallelIterator>::Iter: IndexedParallelIterator,
    P::Subpixel: Send,
{
    fn from(image: PhysicalImage<P, W, H>) -> Self {
        let PhysicalImage { width, height, data } = image;
        let width = width.value() as u32;
        let height = height.value() as u32;
        unsafe {
            let mut raw = Vec::<P::Subpixel>::with_capacity(data.len() * P::CHANNEL_COUNT as usize);
            raw.set_len(data.len() * P::CHANNEL_COUNT as usize);
            raw.par_iter_mut().chunks(P::CHANNEL_COUNT as usize).zip_eq(data.into_par_iter()).for_each(|(ptr, data)| {
                ptr.into_iter().zip(data.channels()).for_each(|(ptr, data)| {
                    let ptr: *mut _ = ptr;
                    ptr.write(*data);
                });
            });
            ImageBuffer::from_raw(width, height, raw).unwrap()
        }
    }
}

impl<P: 'static + Pixel + Send> From<DynamicImage> for PhysicalImage<P, usize, usize>
where
    ImageBuffer<Luma<u8>, Vec<u8>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<LumaA<u8>, Vec<u8>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Rgb<u8>, Vec<u8>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Rgba<u8>, Vec<u8>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Bgr<u8>, Vec<u8>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Bgra<u8>, Vec<u8>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Luma<u16>, Vec<u16>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<LumaA<u16>, Vec<u16>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Rgb<u16>, Vec<u16>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Rgba<u16>, Vec<u16>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Bgr<u16>, Vec<u16>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    ImageBuffer<Bgra<u16>, Vec<u16>>: ConvertBuffer<ImageBuffer<P, Vec<P::Subpixel>>>,
    Vec<P::Subpixel>: IntoParallelIterator<Item = P::Subpixel>,
    <Vec<P::Subpixel> as IntoParallelIterator>::Iter: IndexedParallelIterator,
{
    fn from(image: DynamicImage) -> Self {
        let image: ImageBuffer<P, Vec<P::Subpixel>> = match image {
            DynamicImage::ImageLuma8(image) => image.convert(),
            DynamicImage::ImageLumaA8(image) => image.convert(),
            DynamicImage::ImageRgb8(image) => image.convert(),
            DynamicImage::ImageRgba8(image) => image.convert(),
            DynamicImage::ImageBgr8(image) => image.convert(),
            DynamicImage::ImageBgra8(image) => image.convert(),
            DynamicImage::ImageLuma16(image) => image.convert(),
            DynamicImage::ImageLumaA16(image) => image.convert(),
            DynamicImage::ImageRgb16(image) => image.convert(),
            DynamicImage::ImageRgba16(image) => image.convert(),
        };
        image.into()
    }
}

impl<T, W: MayBeConst<usize>, H: MayBeConst<usize>> ReadPixel for PhysicalImage<T, W, H> {
    type Item = T;

    fn width(&self) -> usize {
        self.width.value()
    }

    fn height(&self) -> usize {
        self.height.value()
    }

    fn is_valid<X: MayBeConst<usize>, Y: MayBeConst<usize>>(&self, x: X, y: Y) -> bool {
        x.value() < self.width.value() && y.value() < self.height.value()
    }

    unsafe fn get_unchecked<X: MayBeConst<usize>, Y: MayBeConst<usize>>(&self, x: X, y: Y) -> &Self::Item {
        debug_assert!(self.is_valid(x, y), "Location ({}, {}) is not valid in PhysicalImage::get_unchecked", x, y);
        self.data.get_unchecked(self.width.value() * y.value() + x.value())
    }
}

impl<T, W: MayBeConst<usize>, H: MayBeConst<usize>> WritePixel for PhysicalImage<T, W, H> {
    unsafe fn get_unchecked_mut<X: MayBeConst<usize>, Y: MayBeConst<usize>>(&mut self, x: X, y: Y) -> &mut Self::Item {
        debug_assert!(self.is_valid(x, y), "Location ({}, {}) is not valid in PhysicalImage::get_unchecked_mut", x, y);
        self.data.get_unchecked_mut(self.width.value() * y.value() + x.value())
    }
}

impl<T, W: MayBeConst<usize>, H: MayBeConst<usize>> View for PhysicalImage<T, W, H> {
    fn view_is_valid<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> bool {
        x + w.value() <= self.width.value() && y + h.value() <= self.height.value()
    }

    unsafe fn view_unchecked<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: usize, y: usize, w: RW, h: RH) -> ImageRef<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in PhysicalImage::view_unchecked",
            x,
            y,
            w,
            h
        );
        ImageRef::new(self.width.value(), self.data.as_ptr(), x, y, w, h)
    }

    fn view_overhang<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhang<T, RW, RH> {
        let valid_x = x.clamp(0, self.width.value() as isize) as usize;
        let valid_y = y.clamp(0, self.height.value() as isize) as usize;
        let valid_width = (x + w.value() as isize).clamp(0, self.width.value() as isize) as usize - valid_x;
        let valid_height = (y + h.value() as isize).clamp(0, self.height.value() as isize) as usize - valid_y;
        ImageRefOverhang::new(
            unsafe { self.view_unchecked(valid_x, valid_y, valid_width, valid_height) },
            (-x).max(0) as usize,
            (-y).max(0) as usize,
            w,
            h,
        )
    }
}

impl<T, W: MayBeConst<usize>, H: MayBeConst<usize>> ViewMut for PhysicalImage<T, W, H> {
    unsafe fn view_unchecked_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: usize, y: usize, w: RW, h: RH) -> ImageRefMut<T, RW, RH> {
        debug_assert!(
            self.view_is_valid(x, y, w, h),
            "Rectangle {{x:{}, y:{}, w:{}, h:{}}} is not valid in PhysicalImage::view_unchecked_mut",
            x,
            y,
            w,
            h
        );
        ImageRefMut::new(self.width.value(), self.data.as_mut_ptr(), x, y, w, h)
    }

    fn view_overhang_mut<RW: MayBeConst<usize>, RH: MayBeConst<usize>>(&mut self, x: isize, y: isize, w: RW, h: RH) -> ImageRefOverhangMut<T, RW, RH> {
        let valid_x = x.clamp(0, self.width.value() as isize) as usize;
        let valid_y = y.clamp(0, self.height.value() as isize) as usize;
        let valid_width = (x + w.value() as isize).clamp(0, self.width.value() as isize) as usize - valid_x;
        let valid_height = (y + h.value() as isize).clamp(0, self.height.value() as isize) as usize - valid_y;
        ImageRefOverhangMut::new(
            unsafe { self.view_unchecked_mut(valid_x, valid_y, valid_width, valid_height) },
            (-x).max(0) as usize,
            (-y).max(0) as usize,
            w,
            h,
        )
    }
}

impl<T: Send, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoPixelIterator for PhysicalImage<T, W, H> {
    type Width = W;
    type Height = H;
    type Item = T;
    type Iter = rayon::vec::IntoIter<T>;

    fn into_pix_iter(self) -> PixIter<Self::Iter, Self::Width, Self::Height> {
        let PhysicalImage { width, height, data } = self;
        PixIter::new(data.into_par_iter(), width, height)
    }
}

impl<T: Send, W: MayBeConst<usize>, H: MayBeConst<usize>> PhysicalImage<T, W, H> {
    pub fn pix_iter_mut(&mut self) -> PixIter<impl ParallelIterator<Item = &mut T> + IndexedParallelIterator, W, H> {
        self.view_mut(0, 0, self.width, self.height).unwrap().pix_iter_mut()
    }
}

impl<T: Sync, W: MayBeConst<usize>, H: MayBeConst<usize>> PhysicalImage<T, W, H> {
    pub fn pix_iter(&self) -> PixIter<impl ParallelIterator<Item = &T> + IndexedParallelIterator, W, H> {
        self.view(0, 0, self.width, self.height).unwrap().pix_iter()
    }
}

impl<T, W: MayBeConst<usize>, H: MayBeConst<usize>> IntoSerializedPixelIterator for PhysicalImage<T, W, H> {
    type Width = W;
    type Height = H;
    type Item = T;
    type Iter = std::vec::IntoIter<T>;

    fn into_pix_iter_serialized(self) -> SerializePixIter<Self::Iter, Self::Width, Self::Height> {
        let PhysicalImage { width, height, data } = self;
        SerializePixIter::new(data.into_iter(), width, height)
    }
}

impl<T, W: MayBeConst<usize>, H: MayBeConst<usize>> PhysicalImage<T, W, H> {
    pub fn pix_iter_serialized(&self) -> SerializePixIter<impl ExactSizeIterator<Item = &T>, W, H> {
        self.view(0, 0, self.width, self.height).unwrap().pix_iter_serialized()
    }

    pub fn pix_iter_serialized_mut(&mut self) -> SerializePixIter<impl ExactSizeIterator<Item = &mut T>, W, H> {
        self.view_mut(0, 0, self.width, self.height).unwrap().pix_iter_serialized_mut()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use image::{ImageBuffer, Rgb};
    use rayon::prelude::IndexedParallelIterator;
    use rayon::prelude::IntoParallelRefMutIterator;
    use rayon::prelude::ParallelIterator;

    use crate::physical_image::PhysicalImage;
    use crate::{IntoPixelIterator, IntoSerializedPixelIterator};
    use crate::{ReadPixel, View, ViewMut, WritePixel};

    #[test]
    fn new_physical_image() {
        let image = PhysicalImage::<Option<()>>::new(10, 10);
        assert_eq!(image.data, vec![None; 100]);
        assert_eq!(image.width(), 10);
        assert_eq!(image.height(), 10);
        let image = PhysicalImage::<i32>::with_default(10, 10, 10);
        assert_eq!(image.data, vec![10; 100]);
        assert_eq!(image.width(), 10);
        assert_eq!(image.height(), 10);

        static DROP_COUNTER: AtomicUsize = AtomicUsize::new(0);
        #[derive(Debug, Default)]
        struct D(Option<()>);
        use std::sync::atomic::Ordering::SeqCst;
        impl Drop for D {
            fn drop(&mut self) {
                DROP_COUNTER.fetch_add(1, SeqCst);
            }
        }
        assert_eq!(DROP_COUNTER.load(SeqCst), 0);
        let image = PhysicalImage::<D>::new(10, 10);
        assert_eq!(DROP_COUNTER.load(SeqCst), 0);
        drop(image);
        assert_eq!(DROP_COUNTER.load(SeqCst), 100);
    }

    #[test]
    fn pixel_physical_image() {
        const WIDTH: usize = 10;
        const HEIGHT: usize = 20;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                assert_eq!(image.get_mut(x, y), Some(&mut 0));
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        for y in 0..HEIGHT {
            assert_eq!(image.get_mut(WIDTH, y), None);
        }
        for x in 0..WIDTH {
            assert_eq!(image.get_mut(x, HEIGHT), None);
        }
        assert_eq!(image.data, (0..WIDTH * HEIGHT).into_iter().collect::<Vec<_>>());
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                assert_eq!(image.get(x, y), Some(&(y * 10 + x)));
            }
        }
        for y in 0..HEIGHT {
            assert_eq!(image.get(10usize, y), None);
        }
        for x in 0..WIDTH {
            assert_eq!(image.get(x, HEIGHT), None);
        }
    }

    #[test]
    fn view_physical_image() {
        // || { // COMPILE ERROR!!
        //     let image = PhysicalImage::<i32>::new(1, 1);
        //     image.view(0, 0, 1usize, 1usize)
        // };
        const WIDTH: usize = 20;
        const HEIGHT: usize = 20;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                assert_eq!(image.get_mut(x, y), Some(&mut 0));
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        assert!(image.view(10, 10, 10, 10).is_some());
        assert!(image.view(10, 10, 10, 11).is_none());
        assert!(image.view(10, 10, 11, 10).is_none());
        assert!(image.view(10, 11, 10, 10).is_none());
        assert!(image.view(11, 10, 10, 10).is_none());
        let view = image.view(5usize, 5usize, WIDTH / 2, HEIGHT / 2).unwrap();
        for y in 0..HEIGHT / 2 {
            for x in 0..WIDTH / 2 {
                assert_eq!(image.get(x + 5, y + 5), view.get(x, y));
            }
        }
        for y in 0..HEIGHT / 2 {
            assert_eq!(view.get(WIDTH / 2, y), None);
        }
        for x in 0..WIDTH / 2 {
            assert_eq!(view.get(x, HEIGHT / 2), None);
        }
        let mut view = image.view_mut(5usize, 5usize, WIDTH / 2, HEIGHT / 2).unwrap();
        // image.view_mut(5, 5, WIDTH / 2, HEIGHT / 2); // COMPILE ERROR!!
        // image.view(5, 5, WIDTH / 2, HEIGHT / 2); // COMPILE ERROR!!
        for y in 0..HEIGHT / 2 {
            for x in 0..WIDTH / 2 {
                assert_eq!(Some(&((y + 5) * WIDTH + x + 5)), view.get(x, y));
            }
        }
        for y in 0..HEIGHT / 2 {
            assert_eq!(view.get(WIDTH / 2, y), None);
        }
        for x in 0..WIDTH / 2 {
            assert_eq!(view.get(x, HEIGHT / 2), None);
        }
        for y in 0..HEIGHT / 2 {
            for x in 0..WIDTH / 2 {
                assert_eq!(Some(&mut ((y + 5) * WIDTH + x + 5)), view.get_mut(x, y));
            }
        }
        for y in 0..HEIGHT / 2 {
            assert_eq!(view.get_mut(WIDTH / 2, y), None);
        }
        for x in 0..WIDTH / 2 {
            assert_eq!(view.get_mut(x, HEIGHT / 2), None);
        }
    }

    #[test]
    fn iter() {
        const WIDTH: usize = 50;
        const HEIGHT: usize = 50;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                assert_eq!(image.get_mut(x, y), Some(&mut 0));
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let mut data = image.data.clone();
        let mut image = image.into_pix_iter().collect_image();
        assert_eq!(data, image.data);
        assert_eq!(image.pix_iter().into_inner().collect::<Vec<_>>(), data.iter().collect::<Vec<_>>());
        assert_eq!(image.pix_iter_mut().into_inner().collect::<Vec<_>>(), data.iter_mut().collect::<Vec<_>>());
        let mut image = image.into_pix_iter_serialized().collect_image();
        assert_eq!(data, image.data);
        assert_eq!(image.pix_iter_serialized().into_inner().collect::<Vec<_>>(), data.iter().collect::<Vec<_>>());
        assert_eq!(image.pix_iter_serialized_mut().into_inner().collect::<Vec<_>>(), data.iter_mut().collect::<Vec<_>>());
    }

    #[test]
    fn image_buffer() {
        const WIDTH: usize = 50;
        const HEIGHT: usize = 50;
        let mut vec = vec![0; WIDTH * HEIGHT * 3];
        vec.par_iter_mut().enumerate().for_each(|(i, ptr)| {
            *ptr = (i & 0xff) as u8;
        });
        let physical: PhysicalImage<Rgb<u8>, _, _> = ImageBuffer::<Rgb<u8>, _>::from_raw(WIDTH as u32, HEIGHT as u32, vec.clone()).unwrap().into();
        let image_buffer: ImageBuffer<Rgb<u8>, _> = physical.into();
        assert_eq!(image_buffer.into_raw(), vec);
    }

    #[test]
    fn view_overhang() {
        const WIDTH: usize = 50;
        const HEIGHT: usize = 50;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let overhang = image.view_overhang(-10, -10, 20usize, 20usize);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x < 10 || y < 10 {
                    assert_eq!(overhang.get(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y - 10) * WIDTH + x - 10)));
                }
            }
        }
        let overhang = image.view_overhang(40, -10, 20usize, 20usize);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x >= 10 || y < 10 {
                    assert_eq!(overhang.get(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y - 10) * WIDTH + x + 40)));
                }
            }
        }
        let overhang = image.view_overhang(-10, 40, 20, 20);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x < 10 || y >= 10 {
                    assert_eq!(overhang.get(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y + 40) * WIDTH + x - 10)));
                }
            }
        }
        let overhang = image.view_overhang(40, 40, 20, 20);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x >= 10 || y >= 10 {
                    assert_eq!(overhang.get(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y + 40) * WIDTH + x + 40)));
                }
            }
        }
        let mut overhang = image.view_overhang_mut(-10, -10, 20usize, 20usize);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x < 10 || y < 10 {
                    assert_eq!(overhang.get(x, y), None);
                    assert_eq!(overhang.get_mut(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y - 10) * WIDTH + x - 10)));
                    assert_eq!(overhang.get_mut(x, y), Some(&mut ((y - 10) * WIDTH + x - 10)));
                }
            }
        }
        let mut overhang = image.view_overhang_mut(40, -10, 20usize, 20usize);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x >= 10 || y < 10 {
                    assert_eq!(overhang.get(x, y), None);
                    assert_eq!(overhang.get_mut(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y - 10) * WIDTH + x + 40)));
                    assert_eq!(overhang.get_mut(x, y), Some(&mut ((y - 10) * WIDTH + x + 40)));
                }
            }
        }
        let mut overhang = image.view_overhang_mut(-10, 40, 20, 20);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x < 10 || y >= 10 {
                    assert_eq!(overhang.get(x, y), None);
                    assert_eq!(overhang.get_mut(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y + 40) * WIDTH + x - 10)));
                    assert_eq!(overhang.get_mut(x, y), Some(&mut ((y + 40) * WIDTH + x - 10)));
                }
            }
        }
        let mut overhang = image.view_overhang_mut(40, 40, 20, 20);
        assert_eq!(overhang.width(), 20);
        assert_eq!(overhang.height(), 20);
        for y in 0..20 {
            for x in 0..20 {
                if x >= 10 || y >= 10 {
                    assert_eq!(overhang.get(x, y), None);
                    assert_eq!(overhang.get_mut(x, y), None);
                } else {
                    assert_eq!(overhang.get(x, y), Some(&((y + 40) * WIDTH + x + 40)));
                    assert_eq!(overhang.get_mut(x, y), Some(&mut ((y + 40) * WIDTH + x + 40)));
                }
            }
        }
        ()
    }
}
