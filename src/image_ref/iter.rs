use std::marker::PhantomData;
use std::ops::Range;

use rayon::iter::plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer};
use rayon::prelude::{IndexedParallelIterator, ParallelIterator};

pub struct Iter<'a, T> {
    ptr: *const T,
    base_width: usize,
    offset_x: usize,
    width: usize,
    range: Range<usize>,
    lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a, T: Sync> Send for Iter<'a, T> {}

unsafe impl<'a, T: Sync> Sync for Iter<'a, T> {}

impl<'a, T> Iter<'a, T> {
    pub(crate) fn new(ptr: *const T, base_width: usize, offset_x: usize, width: usize, range: Range<usize>) -> Self {
        Iter {
            ptr,
            base_width,
            offset_x,
            width,
            range,
            lifetime: Default::default(),
        }
    }
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            return None;
        }
        let y = self.range.start / self.width;
        let x = self.range.start % self.width;
        self.range.start += 1;
        Some(unsafe { &*self.ptr.add(self.base_width * y + self.offset_x + x) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.range.len();
        (len, Some(len))
    }
}

impl<'a, T: 'a> ExactSizeIterator for Iter<'a, T> {}

impl<'a, T: 'a> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            return None;
        }
        self.range.end -= 1;
        let y = self.range.end / self.width;
        let x = self.range.end % self.width;
        Some(unsafe { &*self.ptr.add(self.base_width * y + self.offset_x + x) })
    }
}

impl<'a, T: 'a> Producer for Iter<'a, T>
where
    Self: Send,
{
    type Item = &'a T;
    type IntoIter = Self;

    fn into_iter(self) -> Self::IntoIter {
        self
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let index = self.range.start + index;
        let Iter {
            ptr,
            base_width,
            offset_x,
            width,
            range,
            ..
        } = self;
        (
            Iter {
                ptr,
                base_width,
                offset_x,
                width,
                range: range.start..index,
                lifetime: Default::default(),
            },
            Iter {
                ptr,
                base_width,
                offset_x,
                width,
                range: index..range.end,
                lifetime: Default::default(),
            },
        )
    }
}

impl<'a, T: 'a + Sync> ParallelIterator for Iter<'a, T> {
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> <C as Consumer<Self::Item>>::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }
}

impl<'a, T: 'a + Sync> IndexedParallelIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.range.len()
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> <C as Consumer<Self::Item>>::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> <CB as ProducerCallback<Self::Item>>::Output {
        callback.callback(self)
    }
}

pub struct IterMut<'a, T> {
    ptr: *mut T,
    base_width: usize,
    offset_x: usize,
    width: usize,
    range: Range<usize>,
    lifetime: PhantomData<&'a mut ()>,
}

unsafe impl<'a, T: Send> Send for IterMut<'a, T> {}

unsafe impl<'a, T: Send> Sync for IterMut<'a, T> {}

impl<'a, T> IterMut<'a, T> {
    pub(crate) fn new(ptr: *mut T, base_width: usize, offset_x: usize, width: usize, range: Range<usize>) -> Self {
        IterMut {
            ptr,
            base_width,
            offset_x,
            width,
            range,
            lifetime: Default::default(),
        }
    }
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            return None;
        }
        let y = self.range.start / self.width;
        let x = self.range.start % self.width;
        self.range.start += 1;
        Some(unsafe { &mut *self.ptr.add(self.base_width * y + self.offset_x + x) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.range.len();
        (len, Some(len))
    }
}

impl<'a, T: 'a> ExactSizeIterator for IterMut<'a, T> {}

impl<'a, T: 'a> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            return None;
        }
        self.range.end -= 1;
        let y = self.range.end / self.width;
        let x = self.range.end % self.width;
        Some(unsafe { &mut *self.ptr.add(self.base_width * y + self.offset_x + x) })
    }
}

impl<'a, T: 'a> Producer for IterMut<'a, T>
where
    Self: Send,
{
    type Item = &'a mut T;
    type IntoIter = Self;

    fn into_iter(self) -> Self::IntoIter {
        self
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let index = self.range.start + index;
        let IterMut {
            ptr,
            base_width,
            offset_x,
            width,
            range,
            ..
        } = self;
        (
            IterMut {
                ptr,
                base_width,
                offset_x,
                width,
                range: range.start..index,
                lifetime: Default::default(),
            },
            IterMut {
                ptr,
                base_width,
                offset_x,
                width,
                range: index..range.end,
                lifetime: Default::default(),
            },
        )
    }
}

impl<'a, T: 'a + Send> ParallelIterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn drive_unindexed<C>(self, consumer: C) -> <C as Consumer<Self::Item>>::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }
}

impl<'a, T: 'a + Send> IndexedParallelIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.range.len()
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> <C as Consumer<Self::Item>>::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> <CB as ProducerCallback<Self::Item>>::Output {
        callback.callback(self)
    }
}

pub struct IterOverhang<I> {
    iter: I,
    iter_width: usize,
    iter_height: usize,
    offset_x: usize,
    offset_y: usize,
    width: usize,
    height: usize,
    range: Range<usize>,
}

impl<I> IterOverhang<I> {
    pub(crate) fn new(iter: I, iter_width: usize, iter_height: usize, offset_x: usize, offset_y: usize, width: usize, height: usize) -> Self {
        IterOverhang {
            iter,
            iter_width,
            iter_height,
            offset_x,
            offset_y,
            width,
            height,
            range: 0..width * height,
        }
    }
}

impl<I: ExactSizeIterator + DoubleEndedIterator> Iterator for IterOverhang<I> {
    type Item = Option<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            return None;
        }
        let y = self.range.start / self.width;
        let x = self.range.start % self.width;
        self.range.start += 1;
        if self.offset_x <= x && x < self.offset_x + self.iter_width && self.offset_y <= y && y < self.offset_y + self.iter_height {
            Some(self.iter.next())
        } else {
            Some(None)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.range.len();
        (len, Some(len))
    }
}

impl<I: ExactSizeIterator + DoubleEndedIterator> ExactSizeIterator for IterOverhang<I> {}

impl<I: ExactSizeIterator + DoubleEndedIterator> DoubleEndedIterator for IterOverhang<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            return None;
        }
        self.range.end -= 1;
        let y = self.range.end / self.width;
        let x = self.range.end % self.width;
        if self.offset_x <= x && x < self.offset_x + self.iter_width && self.offset_y <= y && y < self.offset_y + self.iter_height {
            Some(self.iter.next_back())
        } else {
            Some(None)
        }
    }
}

impl<I> IterOverhang<I> {
    fn count_in_rect(&self, range: Range<usize>) -> usize {
        self.count_in_rect_from_0(range.end) - self.count_in_rect_from_0(range.start)
    }
    fn count_in_rect_from_0(&self, to: usize) -> usize {
        if to <= self.offset_y * self.width + self.offset_x {
            return 0;
        }
        if (self.offset_y + self.iter_height - 1) * self.width + self.offset_x + self.iter_width - 1 < to {
            return self.iter_width * self.iter_height;
        }
        let to_x = to % self.width;
        let to_y = to / self.width;
        match (to_x <= self.offset_x, self.offset_x + self.iter_width - 1 < to_x) {
            (true, false) => (to_y - self.offset_y) * self.iter_width,
            (false, true) => (to_y - self.offset_y + 1) * self.iter_width,
            (false, false) => (to_y - self.offset_y) * self.iter_width + to_x - self.offset_x,
            _ => unreachable!(),
        }
    }
}

impl<I: Producer<Item = <I as Iterator>::Item, IntoIter = I> + ExactSizeIterator + DoubleEndedIterator> Producer for IterOverhang<I>
where
    <I as Iterator>::Item: Send,
{
    type Item = Option<<I as Iterator>::Item>;
    type IntoIter = Self;

    fn into_iter(self) -> Self::IntoIter {
        self
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let index = index + self.range.start;
        let iter_split_index = self.count_in_rect(self.range.start..index);
        let IterOverhang {
            iter,
            iter_width,
            iter_height,
            offset_x,
            offset_y,
            width,
            height,
            range,
        } = self;
        let (left, right) = iter.split_at(iter_split_index);
        (
            IterOverhang {
                iter: left,
                iter_width,
                iter_height,
                offset_x,
                offset_y,
                width,
                height,
                range: range.start..index,
            },
            IterOverhang {
                iter: right,
                iter_width,
                iter_height,
                offset_x,
                offset_y,
                width,
                height,
                range: index..range.end,
            },
        )
    }
}

impl<I: Producer<Item = <I as Iterator>::Item, IntoIter = I> + ExactSizeIterator + DoubleEndedIterator> ParallelIterator for IterOverhang<I>
where
    <I as Iterator>::Item: Send,
{
    type Item = Option<<I as Iterator>::Item>;

    fn drive_unindexed<C>(self, consumer: C) -> <C as Consumer<Self::Item>>::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }
}

impl<I: Producer<Item = <I as Iterator>::Item, IntoIter = I> + ExactSizeIterator + DoubleEndedIterator> IndexedParallelIterator for IterOverhang<I>
where
    <I as Iterator>::Item: Send,
{
    fn len(&self) -> usize {
        self.range.len()
    }

    fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> <C as Consumer<Self::Item>>::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> <CB as ProducerCallback<Self::Item>>::Output {
        callback.callback(self)
    }
}

#[cfg(test)]
mod tests {
    use rayon::iter::plumbing::Producer;

    use crate::image_ref::iter::IterOverhang;
    use crate::physical_image::PhysicalImage;
    use crate::{IntoPixelIterator, IntoSerializedPixelIterator, View, ViewMut, WritePixel};

    #[test]
    fn overhang_count_in_rect() {
        let i = IterOverhang::new((), 10, 10, 10, 10, 30, 30);
        for begin in 0..30 * 30 {
            for end in begin..30 * 30 {
                let mut count = 0;
                for i in begin..end {
                    let x = i % 30;
                    let y = i / 30;
                    if (10..20).contains(&x) && (10..20).contains(&y) {
                        count += 1;
                    }
                }
                assert_eq!(i.count_in_rect(begin..end), count);
            }
        }
    }

    #[test]
    fn iter() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let view = image.view(10, 10, 10, 10).unwrap();
        let mut vec = Vec::with_capacity(10 * 10);
        for y in 10..20 {
            for x in 10..20 {
                vec.push(y * WIDTH + x);
            }
        }
        let mut iter = view.pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.into_pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let view = image.view(10, 10, 10, 10).unwrap();
        let mut iter = view.into_pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn iter_split() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let view = image.view(10, 10, 10, 10).unwrap();
        let mut vec = Vec::with_capacity(10 * 10);
        for y in 10..20 {
            for x in 10..20 {
                vec.push(y * WIDTH + x);
            }
        }
        fn equals_recurrent<'a>(iter: impl Producer<Item = &'a usize>, expect: &[usize]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next().unwrap(), &expect[0]);
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent(iter_left, expect_left);
                equals_recurrent(iter_right, expect_right);
            }
        }
        fn equals_recurrent_back<'a>(iter: impl Producer<Item = &'a usize>, expect: &[usize]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next_back().unwrap(), &expect[0]);
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent_back(iter_left, expect_left);
                equals_recurrent_back(iter_right, expect_right);
            }
        }
        equals_recurrent(view.pix_iter().into_inner(), &vec);
        equals_recurrent_back(view.pix_iter().into_inner(), &vec);
        equals_recurrent(view.into_pix_iter().into_inner(), &vec);
        let view = image.view(10, 10, 10, 10).unwrap();
        equals_recurrent_back(view.into_pix_iter().into_inner(), &vec);
    }

    #[test]
    fn iter_mut() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let mut view = image.view_mut(10, 10, 10, 10).unwrap();
        let mut vec = Vec::with_capacity(10 * 10);
        for y in 10..20 {
            for x in 10..20 {
                vec.push(y * WIDTH + x);
            }
        }
        let mut iter = view.pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized_mut().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), &mut vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized_mut().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), &mut vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.into_pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let view = image.view_mut(10, 10, 10, 10).unwrap();
        let mut iter = view.into_pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), &vec[i]);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn iter_mut_split() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let mut view = image.view_mut(10, 10, 10, 10).unwrap();
        let mut vec = Vec::with_capacity(10 * 10);
        for y in 10..20 {
            for x in 10..20 {
                vec.push(y * WIDTH + x);
            }
        }
        fn equals_recurrent<'a>(iter: impl Producer<Item = &'a usize>, expect: &[usize]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next().unwrap(), &expect[0]);
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent(iter_left, expect_left);
                equals_recurrent(iter_right, expect_right);
            }
        }
        fn equals_recurrent_back<'a>(iter: impl Producer<Item = &'a usize>, expect: &[usize]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next_back().unwrap(), &expect[0]);
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent_back(iter_left, expect_left);
                equals_recurrent_back(iter_right, expect_right);
            }
        }
        fn equals_recurrent_mut<'a>(iter: impl Producer<Item = &'a mut usize>, expect: &mut [usize]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next().unwrap(), &mut expect[0]);
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at_mut(half);
                equals_recurrent_mut(iter_left, expect_left);
                equals_recurrent_mut(iter_right, expect_right);
            }
        }
        fn equals_recurrent_mut_back<'a>(iter: impl Producer<Item = &'a mut usize>, expect: &mut [usize]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next_back().unwrap(), &mut expect[0]);
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at_mut(half);
                equals_recurrent_mut_back(iter_left, expect_left);
                equals_recurrent_mut_back(iter_right, expect_right);
            }
        }
        equals_recurrent(view.pix_iter().into_inner(), &vec);
        equals_recurrent_back(view.pix_iter().into_inner(), &vec);
        equals_recurrent_mut(view.pix_iter_mut().into_inner(), &mut vec);
        equals_recurrent_mut_back(view.pix_iter_mut().into_inner(), &mut vec);
        equals_recurrent_mut(view.into_pix_iter().into_inner(), &mut vec);
        let view = image.view_mut(10, 10, 10, 10).unwrap();
        equals_recurrent_mut_back(view.into_pix_iter().into_inner(), &mut vec);
    }

    #[test]
    fn iter_overhang() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let view = image.view(10, 10, 10, 10).unwrap();
        let view = view.view_overhang(-10, -10, 30, 30);
        let mut vec = Vec::with_capacity(30 * 30);
        for y in 0..30 {
            for x in 0..30 {
                if (10..20).contains(&x) && (10..20).contains(&y) {
                    vec.push(Some(y * WIDTH + x));
                } else {
                    vec.push(None);
                }
            }
        }
        let mut iter = view.pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), vec[i].as_ref());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), vec[i].as_ref());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.into_pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), vec[i].as_ref());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let view = image.view(10, 10, 10, 10).unwrap();
        let view = view.view_overhang(-10, -10, 30, 30);
        let mut iter = view.into_pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), vec[i].as_ref());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn iter_overhang_split() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let view = image.view(10, 10, 10, 10).unwrap();
        let view = view.view_overhang(-10, -10, 30, 30);
        let mut vec = Vec::with_capacity(30 * 30);
        for y in 0..30 {
            for x in 0..30 {
                if (10..20).contains(&x) && (10..20).contains(&y) {
                    vec.push(Some(y * WIDTH + x));
                } else {
                    vec.push(None);
                }
            }
        }
        fn equals_recurrent<'a>(iter: impl Producer<Item = Option<&'a usize>>, expect: &[Option<usize>]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next().unwrap(), expect[0].as_ref());
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent(iter_left, expect_left);
                equals_recurrent(iter_right, expect_right);
            }
        }
        fn equals_recurrent_back<'a>(iter: impl Producer<Item = Option<&'a usize>>, expect: &[Option<usize>]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next_back().unwrap(), expect[0].as_ref());
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent_back(iter_left, expect_left);
                equals_recurrent_back(iter_right, expect_right);
            }
        }
        equals_recurrent(view.pix_iter().into_inner(), &vec);
        equals_recurrent_back(view.pix_iter().into_inner(), &vec);
        equals_recurrent(view.into_pix_iter().into_inner(), &vec);
        let view = image.view(10, 10, 10, 10).unwrap();
        let view = view.view_overhang(-10, -10, 30, 30);
        equals_recurrent_back(view.into_pix_iter().into_inner(), &vec);
    }

    #[test]
    fn iter_overhang_mut() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let mut view = image.view_mut(10, 10, 10, 10).unwrap();
        let mut view = view.view_overhang_mut(-10, -10, 30, 30);
        let mut vec = Vec::with_capacity(30 * 30);
        for y in 0..30 {
            for x in 0..30 {
                if (10..20).contains(&x) && (10..20).contains(&y) {
                    vec.push(Some(y * WIDTH + x));
                } else {
                    vec.push(None);
                }
            }
        }
        let mut iter = view.pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), vec[i].as_ref());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), vec[i].as_ref());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized_mut().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), vec[i].as_mut());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.pix_iter_serialized_mut().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), vec[i].as_mut());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = view.into_pix_iter_serialized().into_inner();
        assert_eq!(iter.len(), vec.len());
        for i in 0..vec.len() {
            assert_eq!(iter.next().unwrap(), vec[i].as_mut());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut view = image.view_mut(10, 10, 10, 10).unwrap();
        let view = view.view_overhang_mut(-10, -10, 30, 30);
        let mut iter = view.into_pix_iter_serialized().into_inner();
        for i in (0..vec.len()).rev() {
            assert_eq!(iter.next_back().unwrap(), vec[i].as_mut());
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn iter_overhang_mut_split() {
        const HEIGHT: usize = 30;
        const WIDTH: usize = 30;
        let mut image = PhysicalImage::new(WIDTH, HEIGHT);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                *image.get_mut(x, y).unwrap() = y * WIDTH + x;
            }
        }
        let mut view = image.view_mut(10, 10, 10, 10).unwrap();
        let mut view = view.view_overhang_mut(-10, -10, 30, 30);
        let mut vec = Vec::with_capacity(30 * 30);
        for y in 0..30 {
            for x in 0..30 {
                if (10..20).contains(&x) && (10..20).contains(&y) {
                    vec.push(Some(y * WIDTH + x));
                } else {
                    vec.push(None);
                }
            }
        }
        fn equals_recurrent<'a>(iter: impl Producer<Item = Option<&'a usize>>, expect: &[Option<usize>]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next().unwrap(), expect[0].as_ref());
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent(iter_left, expect_left);
                equals_recurrent(iter_right, expect_right);
            }
        }
        fn equals_recurrent_back<'a>(iter: impl Producer<Item = Option<&'a usize>>, expect: &[Option<usize>]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next_back().unwrap(), expect[0].as_ref());
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at(half);
                equals_recurrent_back(iter_left, expect_left);
                equals_recurrent_back(iter_right, expect_right);
            }
        }
        fn equals_recurrent_mut<'a>(iter: impl Producer<Item = Option<&'a mut usize>>, expect: &mut [Option<usize>]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next().unwrap(), expect[0].as_mut());
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at_mut(half);
                equals_recurrent_mut(iter_left, expect_left);
                equals_recurrent_mut(iter_right, expect_right);
            }
        }
        fn equals_recurrent_mut_back<'a>(iter: impl Producer<Item = Option<&'a mut usize>>, expect: &mut [Option<usize>]) {
            if expect.len() == 1 {
                let mut iter = iter.into_iter();
                assert_eq!(iter.next_back().unwrap(), expect[0].as_mut());
                assert_eq!(iter.next(), None);
                assert_eq!(iter.next_back(), None);
            } else {
                let half = expect.len() / 2;
                let (iter_left, iter_right) = iter.split_at(half);
                let (expect_left, expect_right) = expect.split_at_mut(half);
                equals_recurrent_mut_back(iter_left, expect_left);
                equals_recurrent_mut_back(iter_right, expect_right);
            }
        }
        equals_recurrent(view.pix_iter().into_inner(), &vec);
        equals_recurrent_back(view.pix_iter().into_inner(), &vec);
        equals_recurrent_mut(view.pix_iter_mut().into_inner(), &mut vec);
        equals_recurrent_mut_back(view.pix_iter_mut().into_inner(), &mut vec);
        equals_recurrent_mut(view.into_pix_iter().into_inner(), &mut vec);
        let mut view = image.view_mut(10, 10, 10, 10).unwrap();
        let view = view.view_overhang_mut(-10, -10, 30, 30);
        equals_recurrent_mut_back(view.into_pix_iter().into_inner(), &mut vec);
    }
}
