use std::ops::{RangeFrom, RangeTo, Range};
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

#[derive(Debug, PartialEq)]
pub struct Mesh<Ix> {
    pub xs: Vec<f32>,
    pub ys: Vec<f32>,
    pub zs: Vec<f32>,
    pub faces: Vec<[Ix; 3]>,
}
pub type ObjMesh = Mesh<u32>;

#[derive(Debug, PartialEq)]
enum ObjElement {
    Vertex(f32, f32, f32, f32),
    Face(Vec<(u32, Option<u32>, Option<u32>)>),
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
    let (i, c) = character::complete::one_of("vlf")(i)?;
    match c {
        // v x y z [w]
        'v' => {
            let (i, (x, y, z)) = tuple::<_, _, E, _>((line_lex(float), line_lex(float), line_lex(float)))(i)?;
            let (i, w) = line_lex(opt::<_, _, E, _>(float))(i)?;
            Ok((i, ObjElement::Vertex(x, y, z, w.unwrap_or(1f32))))
        },
        // f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 ...
        'f' => combinator::map(multi::many1(line_lex(face_decl_triple)), |es| ObjElement::Face(es))(i),
        'l' => unimplemented!(),
        _ => unreachable!(),
    }
}

pub fn parse_obj<'a, E>(i: &'a str) -> IResult<&'a str, ObjMesh, E> where E: ParseError<&'a str> {
    let (i, elts) = terminated(many0(lex(parse_obj_line)), preceded(multispace0, eof))(i)?;
    let mut xs = vec![];
    let mut ys = vec![];
    let mut zs = vec![];
    let mut faces = vec![];
    elts.iter().for_each(|e| match e {
        &ObjElement::Vertex(x, y, z, _) => {
            xs.push(x);
            ys.push(y);
            zs.push(z);
        },
        ObjElement::Face(vs) => {
            // TODO(chris): normals and textures
            let [ix, iy, iz] = vs[..] else { todo!() };
            faces.push([ix.0, iy.0, iz.0]);
        }
    });
    Ok((i, ObjMesh { xs, ys, zs, faces }))
}

#[cfg(test)]
mod test {
    use nom::error::ErrorKind;

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
        assert_eq!(parse_obj::<(_, ErrorKind)>(input), Ok(("", ObjMesh{ xs: vec![1.0, 1.0, 1.0], ys: vec![2.0, 2.0, 2.0], zs: vec![3.0, 3.0, 3.0], faces: vec![[1, 2, 3]] })));

        let input = r#"

v 1 2 3

v 1 2 3

v 1 2 3
f 1 2 3

"#;
        assert_eq!(parse_obj::<(_, ErrorKind)>(input), Ok(("", ObjMesh{ xs: vec![1.0, 1.0, 1.0], ys: vec![2.0, 2.0, 2.0], zs: vec![3.0, 3.0, 3.0], faces: vec![[1, 2, 3]] })));

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
        assert_eq!(parse_obj::<(_, ErrorKind)>(input), Ok(("", ObjMesh{ xs: vec![1.0, 1.0, 1.0], ys: vec![2.0, 2.0, 2.0], zs: vec![3.0, 3.0, 3.0], faces: vec![[1, 2, 3], [3, 2, 1]] })));
    }
}
