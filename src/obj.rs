use std::ops::RangeFrom;
use nom::{
    error::ParseError,
    IResult,
    character,
    InputTakeAtPosition,
    AsChar,
    combinator::{self, value, opt},
    Slice,
    InputIter,
    InputLength,
    sequence::{self, terminated, tuple},
    Parser, multi::many_m_n, number::streaming::float
};

pub struct Mesh<Ix> {
    pub vertices: Vec<Ix>,
}
type ObjMesh = Mesh<u32>;

enum ObjParseState {
    Ready,
    ParseVertex,
    ParseLine,
    ParseFace,
}
pub struct ObjParser {
    state: ObjParseState
}

#[derive(Debug, PartialEq)]
enum ObjElement {
    Line(),
    Vertex(f32, f32, f32, f32),
    Face(),
}

fn skip_obj_comment<I, E>() -> impl Fn(I) -> IResult<I, (), E>
    where E: ParseError<I>,
          I: Slice<RangeFrom<usize>> + InputIter + InputLength,
          <I as InputIter>::Item: AsChar {
    use character::streaming::{char, newline};
    move |i: I| value::<I, (), _, E, _>((), terminated(char('#'), newline))(i)
}

fn skip_obj_white<I, E>() -> impl Fn(I) -> IResult<I, (), E>
    where E: ParseError<I>,
          I: InputTakeAtPosition,
          <I as InputTakeAtPosition>::Item: AsChar + Clone {
    move |i: I| value((), character::streaming::multispace0::<I, E>)(i)
}

fn lex<I, O, E>(mut p: impl Parser<I, O, E>) -> impl FnMut(I) -> IResult<I, O, E>
    where E: ParseError<I>,
          I: InputTakeAtPosition + Slice<RangeFrom<usize>> + InputIter + InputLength,
          <I as InputTakeAtPosition>::Item: AsChar + Clone,
          <I as InputIter>::Item: AsChar {
    move |i: I| {
        let (i, ()) = skip_obj_white::<I, E>().parse(i)?;
        let (i, ()) = skip_obj_comment::<I, E>().parse(i)?;
        p.parse(i)
    }
}

fn line_lex<I, O, E>(mut p: impl Parser<I, O, E>) -> impl FnMut(I) -> IResult<I, O, E>
    where E: ParseError<I>,
          I: InputTakeAtPosition,
          <I as InputTakeAtPosition>::Item: AsChar + Clone {
    move |i: I| {
        let (i, _) = character::streaming::space0(i)?;
        p.parse(i)
    }
}

fn parse_obj_line<'a, E>() -> impl FnMut(&'a str) -> IResult<&'a str, ObjElement, E>
    where E: ParseError<&'a str> {
    move |i: &'a str| {
        let (i, c) = character::streaming::one_of("vlf")(i)?;
        match c {
            // v x y z [w]
            'v' => {
                let (i, (x, y, z)) = tuple::<_, _, E, _>((line_lex(float), line_lex(float), line_lex(float)))(i)?;
                let (i, w) = opt::<_, _, E, _>(float)(i)?;
                Ok((i, ObjElement::Vertex(x, y, z, w.unwrap_or(1f32))))
            },
            'l' => todo!(),
            // f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 ...
            'f' => todo!(),
            _ => unreachable!(),
        }
    }
}

fn parse_obj() -> ObjMesh {
    //let f = std::fs::File::open("internet");
    todo!()
}

impl ObjParser {
    pub fn parse_file<E: ParseError<String>>(f: std::fs::File) -> Result<ObjMesh, E> {
        todo!()
    }

    fn parse<'a, E: ParseError<&'a str>>(&self, s: &'a str) -> IResult<&'a str, ObjElement, E> {
        match self.state {
            ObjParseState::Ready => todo!(),
            ObjParseState::ParseVertex => todo!(),
            ObjParseState::ParseLine => todo!(),
            ObjParseState::ParseFace => todo!(),
        }
    }
}

#[cfg(test)]
mod test {
    use nom::error::ErrorKind;

    use super::*;

    #[test]
    pub fn test_parse_line() {
        assert_eq!(parse_obj_line::<(_, ErrorKind)>()("v 1.0 2.0 3.0"), Ok(("", ObjElement::Vertex(1.0, 2.0, 3.0, 1.0))));
    }
}
