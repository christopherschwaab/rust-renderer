// NOTE(chris): this currently assumes the viewer is along the z axis.
use std::{ops::{Index, Sub}, borrow::Borrow};

use obj::TriangleMesh;

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

impl<T> Coord<T, 3> where T: Copy {
    pub fn x(&self) -> T { self.0[0] }
    pub fn y(&self) -> T { self.0[1] }
    pub fn z(&self) -> T { self.0[2] }
}

impl<T> Coord<T, 2> where T: Copy {
    pub fn x(&self) -> T { self.0[0] }
    pub fn y(&self) -> T { self.0[1] }
}

impl<'a, T, const N: usize> Sub<&'a Coord<T, N>> for &'a Coord<T, N> where T: Sub<Output=T> + Copy {
    type Output = Coord<T, N>;

    fn sub(self, ys: Self) -> Self::Output {
        // SAFETY: Everything is of the same size N.
        Coord(std::array::from_fn(|i| *unsafe { self.0.get_unchecked(i) } - *unsafe { ys.0.get_unchecked(i) }))
    }
}

impl<const N: usize> Coord<f32, N> {
    fn norm(&self) -> f32 {
        let sum_of_squares = self.0.iter().fold(0.0, |sum, x| sum + x * x);
        sum_of_squares.sqrt()
    }

    fn normalize(&self) -> Self {
        self.scale(1.0 / self.norm())
    }

    fn scale(&self, c: f32) -> Self {
        Self(self.0.map(|x| c*x))
    }
}

impl Coord<f32, 3> {
    fn cross(&self, v: impl Borrow<Self>) -> Self {
        let [x1, y1, z1] = self.0;
        let &Coord([x2, y2, z2]) = v.borrow();
        Self([y1*z2 - z1*y2, z1*x2 - x1*z2, x1*y2 - y1*x2])
    }

    fn dot(&self, v: impl Borrow<Self>) -> f32 {
        let [x1, y1, z1] = self.0;
        let &Coord([x2, y2, z2]) = v.borrow();
        x1*x2 + y1*y2 + z1*z2
    }
}

pub fn barycentric(t: &Triangle<u32, 2>, p: Coord<u32, 2>) -> Coord<f32, 3> {
    // w1*t[0] + w2*t[1] + w3*t[2] = [x y]
    // w1 + w2 + w3 = 0
    let (x, y) = (p.x() as i32, p.y() as i32);
    let (y1, y2, y3) = (t[0].y() as i32, t[1].y() as i32, t[2].y() as i32);
    let (x1, x2, x3) = (t[0].x() as i32, t[1].x() as i32, t[2].x() as i32);

    let dy2y3 = y2 - y3;
    let dx3x2 = x3 - x2;
    let dx1x3 = x1 - x3;
    let dy1y3 = y1 - y3;
    let d = dy2y3 * dx1x3 + dx3x2 * dy1y3;
    let w1 = (dy2y3 * (x - x3) + dx3x2 * (y - y3)) as f32 / d as f32;
    let w2 = ((y3 - y1) * (x - x3) + dx1x3 * (y - y3)) as f32 / d as f32;
    let w3 = 1.0 - w1 - w2;
    Coord([w1, w2, w3])
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

pub fn reduce_dimension<T, const M: usize, const N: usize>(t: &Triangle<T, M>) -> Triangle<T, N> where T: Copy {
    Triangle([
        Coord(std::array::from_fn(|i| t.0[0][i])),
        Coord(std::array::from_fn(|i| t.0[1][i])),
        Coord(std::array::from_fn(|i| t.0[2][i])),
    ])
}

pub fn project_orthographic(v: &Coord<f32, 3>) -> Coord<f32, 2> {
    Coord([v.x(), v.y()])
}

pub fn project_perspective(v: &Coord<f32, 3>, camera_distance: f32, focal_length: f32) -> Coord<f32, 2> {
    // Eye at p looking at v: t*v + (1 - t)*p
    // TODO(chris): rotation
    let t = focal_length / (camera_distance - v.z());
    Coord([
        t * v.x(),
        t * v.y(),
    ])
}

fn rgb_gray(intensity: f32) -> u32 {
    let rgb_intensity = (intensity * 255.0) as u32;
    0xff << 24 | rgb_intensity << 16 | rgb_intensity << 8 | rgb_intensity
}

#[derive(Debug, PartialEq, Eq)]
pub struct DrawParameters {
    pub depth_test: bool,
    pub draw_depth_buffer: bool,
    pub draw_perspective: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn update_fb(
    draw_parameters: &DrawParameters,
    mesh: &TriangleMesh<u32>,
    z_buffer: &mut [f32],
    fb: &mut [u32],
    fb_width: u32,
    fb_height: u32,
    viewscreen_width: f32,
    viewscreen_height: f32,
    camera_distance: f32,
    focal_length: f32
) {
    fb.fill(0xff000000);
    for [v_ix0, v_ix1, v_ix2] in mesh.face_vertices.iter() {
        let v0 = mesh.vertex(*v_ix0);
        let v1 = mesh.vertex(*v_ix1);
        let v2 = mesh.vertex(*v_ix2);

        let normal = (&v1 - &v0).cross(&v2 - &v0).normalize();
        const LIGHT_DIR: Coord<f32, 3> = Coord([0.0, 0.0, 1.0]);
        let intensity = LIGHT_DIR.dot(normal);
        if intensity > 0.0 {
            let color = rgb_gray(intensity);
            draw_triangle(draw_parameters, &Triangle([v0, v1, v2]), z_buffer, fb, fb_width, fb_height, viewscreen_width, viewscreen_height, camera_distance, focal_length, color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_triangle(
    draw_parameters: &DrawParameters,
    t: &Triangle<f32, 3>,
    z_buffer: &mut [f32],
    fb: &mut [u32],
    fb_width: u32,
    fb_height: u32,
    viewscreen_width: f32,
    viewscreen_height: f32,
    camera_distance: f32,
    focal_length: f32,
    color: u32
) {
    let projected_tri: Triangle<f32, 2> = if draw_parameters.draw_perspective {
        Triangle([
            project_perspective(&t.0[0], camera_distance, focal_length),
            project_perspective(&t.0[1], camera_distance, focal_length),
            project_perspective(&t.0[2], camera_distance, focal_length),
        ])
    } else {
        Triangle([
            project_orthographic(&t.0[0]),
            project_orthographic(&t.0[1]),
            project_orthographic(&t.0[2]),
        ])
    };
    let screen_tri = Triangle([
       ndc2screen(&world2ndc(&projected_tri[0], viewscreen_width, viewscreen_height), fb_width, fb_height),
       ndc2screen(&world2ndc(&projected_tri[1], viewscreen_width, viewscreen_height), fb_width, fb_height),
       ndc2screen(&world2ndc(&projected_tri[2], viewscreen_width, viewscreen_height), fb_width, fb_height),
    ]);
    let bb = screen_tri.bounding_box();

    for y in bb[0].y()..=bb[1].y() {
        for x in bb[0].x()..=bb[1].x() {
            let Coord([w0, w1, w2]) = barycentric(&screen_tri, Coord([x, y]));
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue
            }

            let z = w0*t[0].z() + w1*t[1].z() + w2*t[2].z();
            let ix = (y * fb_width + x) as usize;
            if !draw_parameters.depth_test || z_buffer[ix] < z {
                z_buffer[ix] = z;
                fb[ix] = if draw_parameters.draw_depth_buffer { rgb_gray(z) } else { color };
            }
        }
    }
}

pub fn world2ndc(p: &Coord<f32, 2>, viewscreen_width: f32, viewscreen_height: f32) -> Coord<f32, 2> {
    // NOTE(chris): ndc space is the 2*2 box centered at 0,0
    Coord([
        2.0 * p.x() / viewscreen_width,
        2.0 * p.y() / viewscreen_height,
    ])
}

pub fn ndc2screen(p: &Coord<f32, 2>, fb_width: u32, fb_height: u32) -> Coord<u32, 2> {
    Coord([
        ((p.x() + 1.0) * (fb_width / 2) as f32) as u32,
        ((-p.y() + 1.0) * (fb_height / 2) as f32) as u32,
    ])
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_close {
        ($x:expr, $y:expr, $eps:expr) => {
            let d = $x - $y;
            assert!(
                d <= $eps,
                "assertion failed
  expected: {}={} to be within {} of {}={}
  actual:   {} - {} = {} and {} > {}",
                stringify!($x),
                $x,
                stringify!($eps),
                stringify!($y),
                $y,
                $x,
                $y,
                d,
                d,
                $eps
            )
        }
    }
    pub(crate) use assert_close;

    #[test]
    fn test_barycentric() {
        let t0: Triangle<u32, 2> = [[0, 0], [5, 10], [10, 5]].into();
        let ws0 = barycentric(&t0, Coord([5, 5]));
        assert_close!(ws0[0], 1.0 / 3.0, 0.0001);
        assert_close!(ws0[1], 1.0 / 3.0, 0.0001);
        assert_close!(ws0[2], 1.0 / 3.0, 0.0001);
    }

    #[test]
    fn test_bounding_box() {
        let t0: Triangle<u32, 2> = [[0, 0], [5, 10], [10, 5]].into();
        assert_eq!([Coord([0, 0]), Coord([10, 10])], t0.bounding_box());

        let t1: Triangle<u32, 2> = [[8, 4], [6, 12], [10, 5]].into();
        assert_eq!([Coord([6, 4]), Coord([10, 12])], t1.bounding_box());
    }

    #[test]
    fn test_world2ndc() {
        const VIEWSCREEN_WIDTH: f32 = 200.0;
        const VIEWSCREEN_HEIGHT: f32 = 100.0;

        let top_right: Coord<f32, 2> = Coord([VIEWSCREEN_WIDTH / 2.0, VIEWSCREEN_HEIGHT / 2.0]);
        let top_left: Coord<f32, 2> = Coord([-VIEWSCREEN_WIDTH / 2.0, VIEWSCREEN_HEIGHT / 2.0]);
        let center: Coord<f32, 2> = Coord([0.0, 0.0]);
        let bottom_left: Coord<f32, 2> = Coord([-VIEWSCREEN_WIDTH / 2.0, -VIEWSCREEN_HEIGHT / 2.0]);
        let bottom_right: Coord<f32, 2> = Coord([VIEWSCREEN_WIDTH / 2.0, -VIEWSCREEN_HEIGHT / 2.0]);

        let top_right: Coord<f32, 2> = world2ndc(&top_right, VIEWSCREEN_WIDTH, VIEWSCREEN_HEIGHT);
        let top_left: Coord<f32, 2> = world2ndc(&top_left, VIEWSCREEN_WIDTH, VIEWSCREEN_HEIGHT);
        let center: Coord<f32, 2> = world2ndc(&center, VIEWSCREEN_WIDTH, VIEWSCREEN_HEIGHT);
        let bottom_left: Coord<f32, 2> = world2ndc(&bottom_left, VIEWSCREEN_WIDTH, VIEWSCREEN_HEIGHT);
        let bottom_right: Coord<f32, 2> = world2ndc(&bottom_right, VIEWSCREEN_WIDTH, VIEWSCREEN_HEIGHT);

        assert_close!(top_right.x(), 1.0, 0.0001);
        assert_close!(top_right.y(), 1.0, 0.0001);

        assert_close!(top_left.x(), -1.0, 0.0001);
        assert_close!(top_left.y(), 1.0, 0.0001);

        assert_close!(bottom_left.x(), -1.0, 0.0001);
        assert_close!(bottom_left.y(), -1.0, 0.0001);

        assert_close!(bottom_right.x(), 1.0, 0.0001);
        assert_close!(bottom_right.y(), -1.0, 0.0001);

        assert_close!(center.x(), 0.0, 0.0001);
        assert_close!(center.y(), 0.0, 0.0001);
    }

    #[test]
    fn test_ndc2screen() {
       const FB_WIDTH: u32 = 800;
       const FB_HEIGHT: u32 = 600;

       let top_right: Coord<f32, 2> = Coord([1.0, 1.0]);
       let top_left: Coord<f32, 2> = Coord([-1.0, 1.0]);
       let center: Coord<f32, 2> = Coord([0.0, 0.0]);
       let bottom_left: Coord<f32, 2> = Coord([-1.0, -1.0]);
       let bottom_right: Coord<f32, 2> = Coord([1.0, -1.0]);

       let screen_top_right: Coord<u32, 2> = ndc2screen(&top_right, FB_WIDTH, FB_HEIGHT);
       let screen_top_left: Coord<u32, 2> = ndc2screen(&top_left, FB_WIDTH, FB_HEIGHT);
       let screen_center: Coord<u32, 2> = ndc2screen(&center, FB_WIDTH, FB_HEIGHT);
       let screen_bottom_left: Coord<u32, 2> = ndc2screen(&bottom_left, FB_WIDTH, FB_HEIGHT);
       let screen_bottom_right: Coord<u32, 2> = ndc2screen(&bottom_right, FB_WIDTH, FB_HEIGHT);

       assert_eq!(screen_top_right.x(), 800);
       assert_eq!(screen_top_right.y(), 0);

       assert_eq!(screen_top_left.x(), 0);
       assert_eq!(screen_top_left.y(), 0);

       assert_eq!(screen_center.x(), 800 / 2);
       assert_eq!(screen_center.y(), 600 / 2);

       assert_eq!(screen_bottom_right.x(), 800);
       assert_eq!(screen_bottom_right.y(), 600);

       assert_eq!(screen_bottom_left.x(), 0);
       assert_eq!(screen_bottom_left.y(), 600);
    }
}
