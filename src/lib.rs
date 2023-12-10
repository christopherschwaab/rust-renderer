use std::ops::{Index, Add, Mul, Sub, Div};

pub mod obj;

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coord<T, const N: usize>(pub [T; N]);

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triangle<T, const N: usize>(pub [Coord<T, N>; 3]);

pub type Coord2 = Coord<f32, 2>;
pub type Coord3 = Coord<f32, 3>;
pub type Triangle2 = Triangle<f32, 2>;
pub type Triangle3 = Triangle<f32, 3>;

pub trait NumericUnit {
    fn unit() -> Self;
}

macro_rules! def_unit {
    ($u:expr, $($t:ty),*) => { $( impl NumericUnit for $t { fn unit() -> Self { $u } } )* };
}
def_unit!(1, u8, u16, u32, u64, i8, i16, i32, i64);
def_unit!(1.0, f32, f64);

impl<T, const N: usize> Index<usize> for Triangle<T, N> {
    type Output = Coord<T, N>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T, const N: usize> Index<usize> for Coord<T, N> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T, const N: usize> From<[[T; N]; 3]> for Triangle<T, N> {
    fn from(points: [[T; N]; 3]) -> Self {
        Triangle(points.map(Coord))
    }
}

impl<T> Coord<T, 2> where T: Copy {
    pub fn x(&self) -> T { self.0[0] }
    pub fn y(&self) -> T { self.0[1] }
}

impl<T> Triangle<T, 2>
    where T: Add<Output=T> + Sub<Output=T> + Mul<Output=T> + Div<Output=T> + Copy + NumericUnit {
    pub fn barycentric(&self, p: Coord<T, 2>) -> Coord<T, 3> {
        // w1*t[0] + w2*t[1] + w3*t[2] = [x y]
        // w1 + w2 + w3 = 0
        let (x, y) = (p[0], p[1]);
        let (y1, y2, y3) = (self[0][1], self[1][1], self[2][1]);
        let (x1, x2, x3) = (self[0][0], self[1][0], self[2][0]);

        let dy2y3 = y2 - y3;
        let dx3x2 = x3 - x2;
        let dx1x3 = x1 - x3;
        let dy1y3 = y1 - y3;
        let d = dy2y3 * dx1x3 + dx3x2 * dy1y3;
        let w1 = (dy2y3 * (x - x3) + dx3x2 * (y - y3)) / d;
        let w2 = ((y3 - y1) * (x - x3) + dx1x3 * (y - y3)) / d;
        let w3 = T::unit() - w1 - w2;
        Coord([w1, w2, w3])
    }
}

impl<T> Triangle<T, 2> where T: Ord + Copy {
    pub fn bounding_box(&self) -> [Coord<T, 2>; 2] {
        let min_x = self.0[0].x().min(self.0[1].x()).min(self.0[2].x());
        let min_y = self.0[0].y().min(self.0[1].y()).min(self.0[2].y());
        let max_x = self.0[0].x().max(self.0[1].x()).max(self.0[2].x());
        let max_y = self.0[0].y().max(self.0[1].y()).max(self.0[2].y());
        [Coord([min_x, min_y]), Coord([max_x, max_y])]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_barycentric() {
        let t0: Triangle<f32, 2> = [[0.0, 0.0], [5.0, 10.0], [10.0, 5.0]].into();
        let ws0 = t0.barycentric(Coord([5.0, 5.0]));
        assert_eq!(true, ws0[0] - 1.0 / 3.0 <= 0.0001);
        assert_eq!(true, ws0[1] - 1.0 / 3.0 <= 0.0001);
        assert_eq!(true, ws0[2] - 1.0 / 3.0 <= 0.0001);
    }

    #[test]
    fn test_bounding_box() {
        let t0: Triangle<u32, 2> = [[0, 0], [5, 10], [10, 5]].into();
        assert_eq!([Coord([0, 0]), Coord([10, 10])], t0.bounding_box());

        let t1: Triangle<u32, 2> = [[8, 4], [6, 12], [10, 5]].into();
        assert_eq!([Coord([6, 4]), Coord([10, 12])], t1.bounding_box());
    }
}
