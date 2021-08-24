use nom::bytes::complete::escaped_transform;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::character::complete::char;
use nom::character::complete::one_of;
use nom::character::complete::space1;
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::IResult;

#[derive(Debug, PartialEq)]
pub enum StreamUnit {
    LexicalUnit(Vec<Analysis>),
    Chunk(String),
}

#[derive(Debug, PartialEq)]
pub struct Analysis {
    ling_form: String,
    tags: Vec<String>,
}

pub fn parse_analysis(input: &str) -> IResult<&str, Analysis> {
    let lemma_inner_parse = is_not(r#"^$/<>{}\"#);
    let mut lemma_escape_parse = escaped_transform(lemma_inner_parse, '\\', one_of("^$"));
    lemma_escape_parse(input).map(|(i, o)| {
        (
            i,
            Analysis {
                ling_form: o.to_string(),
                tags: vec![],
            },
        )
    })
}

pub fn parse_lu(input: &str) -> IResult<&str, StreamUnit> {
    let parse_analyses = separated_list0(tag("/"), parse_analysis);
    let mut parse = delimited(char('^'), parse_analyses, char('$'));
    let res = parse(input);
    res.map(|(i, o)| (i, StreamUnit::LexicalUnit(o)))
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
                StreamUnit::LexicalUnit(vec![Analysis {
                    ling_form: String::from("กา"),
                    tags: vec![]
                }])
            ))
        );
    }

    #[test]
    fn lu_surface_escape() {
        assert_eq!(
            parse_lu("^\\^ab\\$$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(vec![Analysis {
                    ling_form: String::from("^ab$"),
                    tags: vec![]
                }])
            ))
        );
    }

    #[test]
    fn ambiguous_lu() {
        assert_eq!(
            parse_lu("^ab/xy$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(vec![
                    Analysis {
                        ling_form: String::from("ab"),
                        tags: vec![]
                    },
                    Analysis {
                        ling_form: String::from("xy"),
                        tags: vec![]
                    }
                ])
            ))
        );
    }

    #[test]
    fn basic_lus() {
        assert_eq!(
            parse_stream("^ab$"),
            Ok((
                "",
                vec![StreamUnit::LexicalUnit(vec![Analysis {
                    ling_form: String::from("ab"),
                    tags: vec![]
                }])]
            ))
        );
        assert_eq!(
            parse_stream("^ab$ ^cd$"),
            Ok((
                "",
                vec![
                    StreamUnit::LexicalUnit(vec![Analysis {
                        ling_form: String::from("ab"),
                        tags: vec![]
                    }]),
                    StreamUnit::LexicalUnit(vec![Analysis {
                        ling_form: String::from("cd"),
                        tags: vec![]
                    }])
                ]
            ))
        );
    }
}
