use std::ops::Index;

pub mod obj;

pub type Coord<const N: usize> = [f32; N];
#[repr(transparent)]
pub struct Triangle<const N: usize>([Coord<N>; N]);

pub type Coord2 = Coord<2>;
pub type Coord3 = Coord<3>;
pub type Triangle2 = Triangle<2>;
pub type Triangle3 = Triangle<3>;

impl<const N: usize> Index<usize> for Triangle<N> {
    type Output = Coord<N>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }

}

impl Triangle2 {
    pub fn barycentric(&self, p: Coord2) -> Coord3 {
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
        let w3 = 1.0f32 - w1 - w2;
        [w1, w2, w3]
    }
}
