use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::MulAssign;
use std::ops::SubAssign;

pub(crate) trait VecMath<T>
where
    T: Float,
{
    fn avg(&self) -> Option<T>;
    fn argmax(&self) -> Option<usize>;
    fn normalize(&mut self);
    fn add(&mut self, other: &[T]);
    fn sub(&mut self, other: &[T]);
    fn scale(&mut self, factor: T);
    fn saturate_lower(&mut self, min: T);
}

impl<T> VecMath<T> for Vec<T>
where
    T: Float,
{
    fn avg(&self) -> Option<T> {
        let mut sum = T::from_usize(0);
        self.iter().for_each(|&item| sum += item);

        Some(sum / T::from_usize(self.len()))
    }

    fn argmax(&self) -> Option<usize> {
        let mut max = *self.first()?;
        let mut argmax: usize = 0;

        for (idx, &item) in self.iter().enumerate().skip(1) {
            if item > max {
                max = item;
                argmax = idx;
            }
        }

        Some(argmax)
    }

    fn normalize(&mut self) {
        let mut sum = T::from_usize(0);
        self.iter().for_each(|&item| sum += item);
        self.iter_mut().for_each(|item| *item /= sum);
    }

    fn add(&mut self, other: &[T]) {
        self.iter_mut().zip(other).for_each(|(a, &b)| *a += b);
    }

    fn sub(&mut self, other: &[T]) {
        self.iter_mut().zip(other).for_each(|(a, &b)| *a -= b);
    }

    fn scale(&mut self, factor: T) {
        self.iter_mut().for_each(|item| *item *= factor);
    }

    fn saturate_lower(&mut self, min: T) {
        self.iter_mut().for_each(|item| {
            if *item < min {
                *item = min
            }
        });
    }
}
pub trait Float:
    PartialOrd + Copy + Div<Output = Self> + AddAssign + SubAssign + DivAssign + MulAssign
{
    fn from_usize(n: usize) -> Self;
}

impl Float for f32 {
    fn from_usize(n: usize) -> Self {
        n as f32
    }
}

impl Float for f64 {
    fn from_usize(n: usize) -> Self {
        n as f64
    }
}
