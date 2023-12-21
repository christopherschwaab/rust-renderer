use std::ops::{RangeFrom, RangeTo, Range};
use log::trace;
use nom::{
    character::{self, complete::{line_ending, not_line_ending, space0, multispace0}},
    error::ParseError,
    IResult,
    InputTakeAtPosition,
    AsChar,
    combinator::{self, value, opt, eof},
    Slice,
    InputIter,
    InputLength,
    sequence::{terminated, tuple, preceded},
    Parser,
    number::complete::float,
    InputTake,
    multi::{self, many0}, Compare, branch::alt
};

use crate::Coord;

//const OPTION_F32_IS_F32_SIZED: () = {
//    const _: [(); 0 - !{ const ASSERT: bool = mem::size_of::<Option<f32>>() == mem::size_of::<f32>(); ASSERT as usize }] = [];
//    ()
//};

#[derive(Debug, PartialEq)]
pub struct TriangleMesh<Ix> {
    pub xs: Vec<f32>,
    pub ys: Vec<f32>,
    pub zs: Vec<f32>,
    pub normals: Vec<Coord<f32, 3>>,
    pub texture_coords: Vec<Coord<f32, 2>>,
    pub face_vertices: Vec<[Ix; 3]>,
    pub face_normals: Vec<[Ix; 3]>,
    pub face_texture_coords: Vec<[Ix; 3]>,
}
pub type ObjTriangleMesh = TriangleMesh<u32>;

#[derive(Debug, PartialEq)]
enum ObjElement {
    Vertex(f32, f32, f32, f32),
    VertexTexture(f32, f32),
    VertexNormal(f32, f32, f32, f32),
    Face(Vec<(u32, Option<u32>, Option<u32>)>),
    Line(),
    Smoothing(bool),
    Group(),
}

pub trait AsUsize {
    #[allow(clippy::wrong_self_convention)]
    fn as_usize(self) -> usize;
}
impl AsUsize for u32 { fn as_usize(self) -> usize { self as usize }}

impl<Ix> TriangleMesh<Ix> {
    pub fn from_vertices(xs: Vec<f32>, ys: Vec<f32>, zs: Vec<f32>, faces: Vec<[Ix; 3]>) -> Self {
        Self {
            xs,
            ys,
            zs,
            normals: vec![],
            texture_coords: vec![],
            face_vertices: faces,
            face_normals: vec![],
            face_texture_coords: vec![],
        }
    }
}

impl<Ix> TriangleMesh<Ix> where Ix: AsUsize + Copy {
    // TODO(chris): probably should actually fixup the indexing to be 0 based at
    // parse time instead of handling it here.
    pub fn vertex(&self, ix: Ix) -> Coord<f32, 3> {
        Coord([self.xs[ix.as_usize() - 1], self.ys[ix.as_usize() - 1], self.zs[ix.as_usize() - 1]])
    }

    pub fn texture_coords(&self, ix: Ix) -> Coord<f32, 2> {
        self.texture_coords[ix.as_usize() - 1].clone()
    }
    pub fn normal(&self, ix: Ix) -> Coord<f32, 3> {
        self.normals[ix.as_usize() - 1].clone()
    }
}

fn rest_of_line<I, E>(i: I) -> IResult<I, (), E>
    where E: ParseError<I>,
          I: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>> + InputTakeAtPosition + InputTake + InputIter + InputLength + Compare<&'static str> + Clone,
          <I as InputIter>::Item: AsChar,
          <I as InputTakeAtPosition>::Item: AsChar {
    value((), terminated(not_line_ending, alt((line_ending, eof))))(i)
}

fn obj_comment<I, E>(i: I) -> IResult<I, (), E>
    where E: ParseError<I>,
          I: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>> + InputTakeAtPosition + InputIter + InputTake + InputLength + Clone + nom::Compare<&'static str>,
          <I as InputTakeAtPosition>::Item: AsChar,
          <I as InputIter>::Item: AsChar {
    use nom::character::complete::char;
    value(
        (),
        preceded(
            char('#'),
            rest_of_line
        )
    )(i)
}

fn obj_white<I, E>(i: I) -> IResult<I, (), E>
    where E: ParseError<I>,
          I: InputTakeAtPosition,
          <I as InputTakeAtPosition>::Item: AsChar + Clone {
    value((), multispace0::<I, E>)(i)
}

fn lex<I, O, E>(mut p: impl Parser<I, O, E>) -> impl FnMut(I) -> IResult<I, O, E>
    where E: ParseError<I>,
          I: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>> + InputTakeAtPosition + InputIter + InputTake + InputLength + Clone + nom::Compare<&'static str>,
          <I as InputTakeAtPosition>::Item: AsChar + Clone,
          <I as InputIter>::Item: AsChar {
    move |i: I| {
        let (i, _) = many0(preceded(obj_white::<_, E>, obj_comment))(i)?;
        let (i, _) = obj_white(i)?;
        p.parse(i)
    }
}

fn line_lex<I, O, E>(p: impl Parser<I, O, E>) -> impl FnMut(I) -> IResult<I, O, E>
    where E: ParseError<I>,
          I: InputTakeAtPosition,
          <I as InputTakeAtPosition>::Item: AsChar + Clone {
    preceded(space0, p)
}

fn face_decl_triple<I, E>(i: I) -> IResult<I, (u32, Option<u32>, Option<u32>), E>
    where E: ParseError<I>,
          I: InputIter + Slice<RangeFrom<usize>> + InputLength + Clone,
          <I as InputIter>::Item: AsChar {
    use character::complete::{char, u32 as obj_index};
    let (i, v_ix) = obj_index.parse(i)?;
    match opt(char::<_, E>('/')).parse(i)? {
        (i, Some(_)) => match opt(obj_index).parse(i)? {
            (i, Some(vt_ix)) => match opt(char::<_, E>('/')).parse(i)? {
                (i, Some(_)) => {
                    let (i, vn_ix) = obj_index.parse(i)?;
                    Ok((i, (v_ix, Some(vt_ix), Some(vn_ix))))
                },
                (i, None) => Ok((i, (v_ix, Some(vt_ix), None))),
            },
            (i, None) => {
                let (i, _) = char::<_, E>('/').parse(i)?;
                let (i, vn_ix) = obj_index.parse(i)?;
                Ok((i, (v_ix, None, Some(vn_ix))))
            },
        },
        (i, None) => Ok((i, (v_ix, None, None))),
    }
}

fn parse_obj_line<'a, E>(i: &'a str) -> IResult<&'a str, ObjElement, E> where E: ParseError<&'a str> {
    let (i, c) = character::complete::one_of("vlfgs")(i)?;
    match c {
        // v x y z [w]
        // vt x y z [w]
        // vn x y z [w]
        'v' => {
            match opt(character::complete::one_of("tn")).parse(i)? {
                (i, Some('t')) => {
                    let (i, (u, v)) = tuple::<_, _, E, _>((line_lex(float), line_lex(float)))(i)?;
                    let (i, _) = rest_of_line(i)?;
                    Ok((i, ObjElement::VertexTexture(u, v)))
                },
                (i, Some('n')) => {
                    let (i, (x, y, z)) = tuple::<_, _, E, _>((line_lex(float), line_lex(float), line_lex(float)))(i)?;
                    let (i, w) = line_lex(opt::<_, _, E, _>(float))(i)?;
                    Ok((i, ObjElement::VertexNormal(x, y, z, w.unwrap_or(1f32))))
                },
                (i, None) => {
                    let (i, (x, y, z)) = tuple::<_, _, E, _>((line_lex(float), line_lex(float), line_lex(float)))(i)?;
                    let (i, w) = line_lex(opt::<_, _, E, _>(float))(i)?;
                    Ok((i, ObjElement::Vertex(x, y, z, w.unwrap_or(1f32))))
                },
                (_, Some(_)) => unreachable!(),
            }
        },
        // f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 ...
        'f' => combinator::map(multi::many1(line_lex(face_decl_triple)), ObjElement::Face)(i),
        'l' => {
            trace!("STUB: ignoring obj line element");
            Ok((i, ObjElement::Line()))
        },
        'g' => {
            trace!("STUB: ignoring obj group element");
            let (i, _) = rest_of_line(i)?;
            Ok((i, ObjElement::Group()))
        },
        's' => {
            let (i, c) = line_lex(character::complete::one_of("01"))(i)?;
            Ok((i, ObjElement::Smoothing((c as u8 - b'0') != 0)))
        },
        c => unimplemented!("obj element type {}", c),
    }
}

pub fn parse_obj<'a, E>(i: &'a str) -> IResult<&'a str, ObjTriangleMesh, E> where E: ParseError<&'a str> {
    let (i, elts) = terminated(many0(lex(parse_obj_line)), lex(eof))(i)?;
    let mut xs = vec![];
    let mut ys = vec![];
    let mut zs = vec![];
    let mut normals: Vec<Coord<f32, 3>> = vec![];
    let mut face_vertices: Vec<[u32; 3]> = vec![];
    let mut texture_coords: Vec<Coord<f32, 2>> = vec![];
    let mut face_normals: Vec<[u32; 3]> = vec![];
    let mut face_texture_coords: Vec<[u32; 3]> = vec![];

    elts.iter().for_each(|e| match e {
        &ObjElement::Vertex(x, y, z, _) => {
            xs.push(x);
            ys.push(y);
            zs.push(z);
        },
        ObjElement::Face(vs) => {
            if vs.len() != 3 {
                // TODO(chris): fail gracefully with a recoverable error of some sort
                panic!("invalid triangel mesh, faces must consist of 3 vertices");
            }
            let [(v0, vt0, vn0), (v1, vt1, vn1), (v2, vt2, vn2)] = vs[..] else { unreachable!() };

            face_vertices.push([v0, v1, v2]);
            match (vt0, vt1, vt2) {
                (None, None, None) => {},
                (Some(vt0), Some(vt1), Some(vt2)) => {
                    face_texture_coords.push([vt0, vt1, vt2]);
                },
                _ => {
                    // TODO(chris): is this true?
                    panic!("invalid face texture spec, each vertex of a face must be given a coordinate");
                },
            }
            match (vn0, vn1, vn2) {
                (None, None, None) => {},
                (Some(vn0), Some(vn1), Some(vn2)) => {
                    face_normals.push([vn0, vn1, vn2]);
                },
                _ => {
                    // TODO(chris): is this true?
                    panic!("invalid face normal spec, each vertex of a face must be given a normal");
                },
            }
        },
        &ObjElement::VertexTexture(u, v) => {
            texture_coords.push(Coord([u, v]));
        },
        &ObjElement::VertexNormal(x, y, z, _) => {
            normals.push(Coord([x, y, z]));
        },
        &ObjElement::Line() => todo!(),
        &ObjElement::Smoothing(_) => {
            trace!("STUB: ignoring smoothing toogle in mesh construction");
        },
        &ObjElement::Group() => {
            trace!("STUB: ignoring groups specifier in mesh construction");
        },
    });
    Ok((i, ObjTriangleMesh {
        xs,
        ys,
        zs,
        normals,
        texture_coords,
        face_vertices,
        face_normals,
        face_texture_coords,
    }))
}

#[cfg(test)]
mod test {
    use nom::error::ErrorKind;

    use crate::test::assert_close;

    use super::*;

    #[test]
    pub fn test_vertex_parse_line() {
        assert_eq!(line_lex::<_, _, (_, ErrorKind)>(float)("1.0\n"), Ok(("\n", 1.0)));

        // NOTE(chris): eat trailing spaces because of the optional (space preceded) w.
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("v 1 2 3\n"), Ok(("\n", ObjElement::Vertex(1.0, 2.0, 3.0, 1.0))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("v 1.0 2.0 3.0\n"), Ok(("\n", ObjElement::Vertex(1.0, 2.0, 3.0, 1.0))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("v    1.0    2.0    3.0   \n"), Ok(("\n", ObjElement::Vertex(1.0, 2.0, 3.0, 1.0))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("v\t1.0  \t\t 2.0\t \t3.0\t\n"), Ok(("\n", ObjElement::Vertex(1.0, 2.0, 3.0, 1.0))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("v 1.0 2.0 3.0 4.0\n"), Ok(("\n", ObjElement::Vertex(1.0, 2.0, 3.0, 4.0))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("v    1.0   2e6     -3.0    \t   4.0\n"), Ok(("\n", ObjElement::Vertex(1.0, 2000000.0, -3.0, 4.0))));
    }

    #[test]
    pub fn test_face_parse_line() {
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("f 1 2 3\n"), Ok(("\n", ObjElement::Face(vec![(1, None, None), (2, None, None), (3, None, None)]))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("f    1    2    3\n"), Ok(("\n", ObjElement::Face(vec![(1, None, None), (2, None, None), (3, None, None)]))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("f  \t  1\t2 \t \t  3\n"), Ok(("\n", ObjElement::Face(vec![(1, None, None), (2, None, None), (3, None, None)]))));

        assert_eq!(parse_obj_line::<(_, ErrorKind)>("f 1/2 2//3 3/1/4\n"), Ok(("\n", ObjElement::Face(vec![(1, Some(2), None), (2, None, Some(3)), (3, Some(1), Some(4))]))));
        assert_eq!(parse_obj_line::<(_, ErrorKind)>("f 1/2 2//3 3/1/4 4/1\n"), Ok(("\n", ObjElement::Face(vec![(1, Some(2), None), (2, None, Some(3)), (3, Some(1), Some(4)), (4, Some(1), None)]))));
    }

    #[test]
    pub fn test_face_decl_triple() {
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("1\n"), Ok(("\n", (1, None, None))));
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("1/2\n"), Ok(("\n", (1, Some(2), None))));
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("1/2/3\n"), Ok(("\n", (1, Some(2), Some(3)))));
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("1//3\n"), Ok(("\n", (1, None, Some(3)))));

        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("123\n"), Ok(("\n", (123, None, None))));
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("1634/2254\n"), Ok(("\n", (1634, Some(2254), None))));
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("121/225/3245\n"), Ok(("\n", (121, Some(225), Some(3245)))));
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("6541//33463\n"), Ok(("\n", (6541, None, Some(33463)))));

        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("6541/\n"), Err(nom::Err::Error(("\n", ErrorKind::Char))));
        assert_eq!(face_decl_triple::<_, (_, ErrorKind)>("6541//\n"),Err(nom::Err::Error(("\n", ErrorKind::Digit))));
    }

    #[test]
    pub fn test_lex() {
        let input = r#"
   #lksjdflj slkjdflkjsdlfkj
   # jjjjdflj slkjdflkjsdlfkj
             1.2
"#;
        assert_eq!(lex::<_, _, (_, ErrorKind)>(float)(input), Ok(("\n", 1.2)));
    }

    #[test]
    pub fn test_parse_obj() {
        let input = r#"
v 1 2 3
v 1 2 3
v 1 2 3
f 1 2 3
"#;
        assert_eq!(parse_obj::<(_, ErrorKind)>(input), Ok(("", ObjTriangleMesh::from_vertices(vec![1.0, 1.0, 1.0], vec![2.0, 2.0, 2.0], vec![3.0, 3.0, 3.0], vec![[1, 2, 3]]))));

        let input = r#"

v 1 2 3

v 1 2 3

v 1 2 3
f 1 2 3

"#;
        assert_eq!(parse_obj::<(_, ErrorKind)>(input), Ok(("", ObjTriangleMesh::from_vertices(vec![1.0, 1.0, 1.0], vec![2.0, 2.0, 2.0], vec![3.0, 3.0, 3.0], vec![[1, 2, 3]]))));

        let input = r#"
  # lskljsdf lskjdflk lkjslakfjd %"^$£"

v 1 2 3

v 1 2 3 # internet
  # internet lksjdflkjl
v 1 2 3 # internet
f 1 2 3

  # internet lksjdflkjl
f 3 2 1
"#;
        assert_eq!(parse_obj::<(_, ErrorKind)>(input), Ok(("", ObjTriangleMesh::from_vertices(vec![1.0, 1.0, 1.0], vec![2.0, 2.0, 2.0], vec![3.0, 3.0, 3.0], vec![[1, 2, 3], [3, 2, 1]]))));

        let input = r#"
# 1258 vertex normals

g head
s 1
f 24/1/42 25/2/29 26/3/27
# 2492 faces
"#;
        let expected = ObjTriangleMesh {
            xs: vec![],
            ys: vec![],
            zs: vec![],
            normals: vec![],
            texture_coords: vec![],
            face_vertices: vec![[24, 25, 26]],
            face_normals: vec![[42, 29, 27]],
            face_texture_coords: vec![[1, 2, 3]],
        };
        assert_eq!(parse_obj::<(_, ErrorKind)>(input), Ok(("", expected)));
    }

    static AFRICAN_HEAD_OBJ: std::sync::OnceLock<TriangleMesh<u32>> = std::sync::OnceLock::new();
    fn get_african_head_obj() -> &'static TriangleMesh<u32> {
        AFRICAN_HEAD_OBJ.get_or_init(|| {
            const AFRICAN_HEAD_OBJ: &str = include_str!("../obj/african_head.obj");

            let (i, obj) = parse_obj::<(_, ErrorKind)>(AFRICAN_HEAD_OBJ).expect("unexpectedly failed to parse african_head.obj");
            assert_eq!("", i);

            obj
        })
    }

    #[test]
    pub fn test_parse_african_head_obj_vertices() {
        let obj = get_african_head_obj();

        let v1 = obj.vertex(1);
        const V1_EXPECTED: Coord<f32, 3> = Coord([-0.000581696, -0.734665, -0.623267]);
        assert_close!(v1.x(), V1_EXPECTED.x(), 1e-6);
        assert_close!(v1.y(), V1_EXPECTED.y(), 1e-6);
        assert_close!(v1.z(), V1_EXPECTED.z(), 1e-6);

        let v1023 = obj.vertex(1024);
        const V1023: Coord<f32, 3> = Coord([-0.101867, 0.0715163, 0.586237]);
        assert_close!(v1023.x(), V1023.x(), 1e-6);
        assert_close!(v1023.y(), V1023.y(), 1e-6);
        assert_close!(v1023.z(), V1023.z(), 1e-6);

        const NUM_VERTICES: usize = 1258;
        let num_vertices = obj.xs.len();
        assert_eq!(num_vertices, NUM_VERTICES);

        let v_last = obj.vertex(num_vertices as u32);
        const V_LAST: Coord<f32, 3> = Coord([-0.171097, 0.299996, 0.415616]);
        assert_eq!(v_last.x(), V_LAST.x());

        let first_vertex = obj.vertex(1);
        assert_close!(first_vertex.x(), -0.000581696, 1e-6);
        assert_close!(first_vertex.y(), -0.734665, 1e-6);
        assert_close!(first_vertex.z(), -0.623267, 1e-6);
    }

    #[test]
    pub fn test_parse_african_head_obj_faces() {
        let obj = get_african_head_obj();

        const NUM_FACES: usize = 2492;
        assert_eq!(obj.face_vertices.len(), NUM_FACES);

        assert_eq!(obj.face_vertices[0], [24, 25, 26]);
        assert_eq!(obj.face_vertices[1171], [630, 663, 670]);
        assert_eq!(obj.face_vertices[NUM_FACES - 1], [1201, 1202, 1200]);

        assert_eq!(obj.face_normals[0], [24, 25, 26]);
        assert_eq!(obj.face_normals[1171], [630, 663, 670]);
        assert_eq!(obj.face_normals[NUM_FACES - 1], [1201, 1202, 1200]);

        assert_eq!(obj.face_texture_coords[0], [24, 25, 26]);
        assert_eq!(obj.face_texture_coords[1171], [630, 663, 670]);
        assert_eq!(obj.face_texture_coords[NUM_FACES - 1], [1201, 1202, 1200]);
    }

    #[test]
    pub fn test_parse_african_head_obj_face_normals() {
        let obj = get_african_head_obj();

        const NUM_NORMALS : usize = 1258;
        assert_eq!(obj.normals.len(), NUM_NORMALS);

        const N0_EXPECTED: Coord<f32, 3> = Coord([0.001, 0.482, -0.876]);
        let n0 = &obj.normals[0];
        assert_close!(n0.x(), N0_EXPECTED.x(), 1e-6);
        assert_close!(n0.y(), N0_EXPECTED.y(), 1e-6);
        assert_close!(n0.z(), N0_EXPECTED.z(), 1e-6);

        const N800_EXPECTED: Coord<f32, 3> = Coord([-0.721, 0.489, 0.490]);
        let n800 = &obj.normals[799];
        assert_close!(n800.x(), N800_EXPECTED.x(), 1e-6);
        assert_close!(n800.y(), N800_EXPECTED.y(), 1e-6);
        assert_close!(n800.z(), N800_EXPECTED.z(), 1e-6);

        const NLAST_EXPECTED: Coord<f32, 3> = Coord([-0.319, -0.065, 0.946]);
        let nlast = &obj.normals[NUM_NORMALS - 1];
        assert_close!(nlast.x(), NLAST_EXPECTED.x(), 1e-6);
        assert_close!(nlast.y(), NLAST_EXPECTED.y(), 1e-6);
        assert_close!(nlast.z(), NLAST_EXPECTED.z(), 1e-6);
    }

    #[test]
    pub fn test_parse_african_head_obj_face_texture_coords() {
        let obj = get_african_head_obj();

        const NUM_TEXTURE_COORDS: usize = 1339;
        assert_eq!(obj.texture_coords.len(), NUM_TEXTURE_COORDS);

        const T1_EXPECTED: Coord<f32, 2> = Coord([0.532, 0.923]);
        let t1 = obj.texture_coords(1);
        assert_close!(t1.x(), T1_EXPECTED.x(), 1e-6);
        assert_close!(t1.y(), T1_EXPECTED.y(), 1e-6);

        const T762_EXPECTED: Coord<f32, 2> = Coord([0.396, 0.718]);
        let t762 = obj.texture_coords(762);
        assert_close!(t762.x(), T762_EXPECTED.x(), 1e-6);
        assert_close!(t762.y(), T762_EXPECTED.y(), 1e-6);

        const TLAST_EXPECTED: Coord<f32, 2> = Coord([0.412, 0.975]);
        let tlast = obj.texture_coords(NUM_TEXTURE_COORDS as u32);
        assert_close!(tlast.x(), TLAST_EXPECTED.x(), 1e-6);
        assert_close!(tlast.y(), TLAST_EXPECTED.y(), 1e-6);

        obj.face_texture_coords
    }
}
