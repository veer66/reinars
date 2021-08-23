//use nom::sequence::preceded;
//use nom::character::complete::digit1;

use nom::character::complete::alpha1;
use nom::character::complete::char;
use nom::character::complete::space1;
use nom::character::complete::alphanumeric1;
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::IResult;
use nom::bytes::complete::take_till1;
use nom::bytes::complete::escaped;
use nom::bytes::complete::escaped_transform;
use nom::character::complete::one_of;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::combinator::cut;

#[derive(Debug, PartialEq)]
pub enum StreamUnit {
    LexicalUnit(LexicalUnit),
    Chunk(String),
}

#[derive(Debug, PartialEq)]
pub enum Analysis {
    SurfaceForm(String),
    Lemma(String),
}

#[derive(Debug, PartialEq)]
pub struct LexicalUnit {
    pub analyses: Vec<Analysis>,
}

pub fn parse_surface_form(input: &str) -> IResult<&str, Analysis> {
    let inner_parse = is_not(r#"^$/<>{}\"#);
    let mut parse = escaped_transform(inner_parse, '\\', one_of("^$"));
    parse(input).map(|(i, o)| (i, Analysis::SurfaceForm(o.to_owned())))
}

pub fn parse_lu(input: &str) -> IResult<&str, StreamUnit> {
    let mut parse = delimited(char('^'), parse_surface_form, char('$'));
    let res = parse(input);
    res.map(|(i, o)| {
        (
            i,
            StreamUnit::LexicalUnit(LexicalUnit {
                analyses: vec![o],
            }),
        )
    })
}

pub fn parse_stream(input: &str) -> IResult<&str, Vec<StreamUnit>> {
    let mut parse = separated_list0(space1, parse_lu);
    parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lu() {
        assert_eq!(
            parse_lu("^กา$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(LexicalUnit {
                    analyses: vec![Analysis::SurfaceForm(String::from("กา"))]
                })
            ))
        );
    }

    #[test]
    fn lu_surface_escape() {
        assert_eq!(
            parse_lu("^\\^ab\\$$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(LexicalUnit {
                    analyses: vec![Analysis::SurfaceForm(String::from("^ab$"))]
                })
            ))
        );
    }

    #[test]
    fn basic_lus() {
        assert_eq!(
            parse_stream("^ab$"),
            Ok((
                "",
                vec![StreamUnit::LexicalUnit(LexicalUnit {
                    analyses: vec![Analysis::SurfaceForm(String::from("ab"))]
                })]
            ))
        );
        assert_eq!(
            parse_stream("^ab$ ^cd$"),
            Ok((
                "",
                vec![
                    StreamUnit::LexicalUnit(LexicalUnit {
                        analyses: vec![Analysis::SurfaceForm(String::from("ab"))]
                    }),
                    StreamUnit::LexicalUnit(LexicalUnit {
                        analyses: vec![Analysis::SurfaceForm(String::from("cd"))]
                    })
                ]
            ))
        );
    }
}
