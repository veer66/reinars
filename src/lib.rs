use nom::branch::alt;
use nom::bytes::complete::escaped_transform;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::character::complete::one_of;
use nom::character::complete::space1;
use nom::multi::many0;
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::sequence::pair;
use nom::IResult;

#[derive(Debug, PartialEq)]
pub struct SubLU {
    ling_form: String,
    tags: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum StreamUnit {
    LexicalUnit(Vec<SubLU>),
    Space(String),
    Format(String),
    JoinedLexicalUnit(Vec<Vec<SubLU>>),
    Chunk(SubLU, Vec<StreamUnit>),
}

pub fn parse_tag(input: &str) -> IResult<&str, &str> {
    let mut parse = delimited(tag("<"), is_not(r#"<>"#), tag(">"));
    parse(input)
}

pub fn parse_basic_lu(input: &str) -> IResult<&str, StreamUnit> {
    let parse_analyses = separated_list0(tag("/"), parse_sub_lu);
    let mut parse = delimited(tag("^"), parse_analyses, tag("$"));
    let res = parse(input);
    res.map(|(i, o)| (i, StreamUnit::LexicalUnit(o)))
}

pub fn parse_sub_lu(input: &str) -> IResult<&str, SubLU> {
    let ling_form_inner_parse = is_not(r#"^$/<>{}\"#);
    let ling_form_escape_parse = escaped_transform(ling_form_inner_parse, '\\', one_of("^$"));
    let mut parse = pair(ling_form_escape_parse, many0(parse_tag));
    parse(input).map(|(i, (ling_form, tags))| {
        (
            i,
            SubLU {
                ling_form: ling_form.to_string(),
                tags: tags.iter().map(|tag| String::from(*tag)).collect(),
            },
        )
    })
}

pub fn parse_joined_lu(input: &str) -> IResult<&str, StreamUnit> {
    let parse_sub_lus = separated_list0(tag("+"), parse_sub_lu);
    let parse_analyses = separated_list0(tag("/"), parse_sub_lus);
    let mut parse = delimited(tag("^"), parse_analyses, tag("$"));
    let res = parse(input);
    res.map(|(i, o)| (i, StreamUnit::JoinedLexicalUnit(o)))
}

pub fn parse_lu_or_space_or_format(input: &str) -> IResult<&str, StreamUnit> {
    alt((parse_format, parse_basic_lu, parse_joined_lu, parse_space))(input)
}

pub fn parse_chunk(input: &str) -> IResult<&str, StreamUnit> {
    let parse_children = delimited(tag("{"), many0(parse_lu_or_space_or_format), tag("}"));
    let mut parse = pair(parse_sub_lu, parse_children);
    let res = parse(input);
    res.map(|(i, (head, children))| (i, StreamUnit::Chunk(head, children)))
}

pub fn parse_format(input: &str) -> IResult<&str, StreamUnit> {
    let mut parse = delimited(tag("["), is_not(r#"[]"#), tag("]"));
    let res = parse(input);
    res.map(|(i, o)| (i, StreamUnit::Format(String::from(o))))
}

pub fn parse_space(input: &str) -> IResult<&str, StreamUnit> {
    space1(input).map(|(i, o)| (i, StreamUnit::Space(String::from(o))))
}

pub fn parse_stream_unit(input: &str) -> IResult<&str, StreamUnit> {
    alt((
        parse_space,
        parse_format,
        parse_basic_lu,
        parse_joined_lu,
        parse_chunk,
    ))(input)
}

pub fn parse_stream(input: &str) -> IResult<&str, Vec<StreamUnit>> {
    let mut parse = many0(parse_stream_unit);
    parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lu() {
        assert_eq!(
            parse_stream_unit("^กา$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(vec![SubLU {
                    ling_form: String::from("กา"),
                    tags: vec![]
                }])
            ))
        );
    }

    #[test]
    fn lu_surface_escape() {
        assert_eq!(
            parse_stream_unit("^\\^ab\\$$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(vec![SubLU {
                    ling_form: String::from("^ab$"),
                    tags: vec![]
                }])
            ))
        );
    }

    #[test]
    fn ambiguous_lu() {
        assert_eq!(
            parse_stream_unit("^ab/xy$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(vec![
                    SubLU {
                        ling_form: String::from("ab"),
                        tags: vec![]
                    },
                    SubLU {
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
                vec![StreamUnit::LexicalUnit(vec![SubLU {
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
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("ab"),
                        tags: vec![]
                    }]),
                    StreamUnit::Space(String::from(" ")),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![]
                    }])
                ]
            ))
        );
    }

    #[test]
    fn parse_basic_tag() {
        assert_eq!(parse_tag("<n>"), Ok(("", "n")));
    }

    #[test]
    fn parse_basic_stream_with_tags() {
        assert_eq!(
            parse_stream("^ab/xy<n>$ ^cd$"),
            Ok((
                "",
                vec![
                    StreamUnit::LexicalUnit(vec![
                        SubLU {
                            ling_form: String::from("ab"),
                            tags: vec![]
                        },
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")]
                        }
                    ]),
                    StreamUnit::Space(String::from(" ")),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![]
                    }])
                ]
            ))
        );
    }

    #[test]
    fn parse_basic_stream_with_tags_sans_space() {
        assert_eq!(
            parse_stream("^ab/xy<n>$^cd$"),
            Ok((
                "",
                vec![
                    StreamUnit::LexicalUnit(vec![
                        SubLU {
                            ling_form: String::from("ab"),
                            tags: vec![]
                        },
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")]
                        }
                    ]),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![]
                    }])
                ]
            ))
        );
    }

    #[test]
    fn parse_basic_stream_with_tags_sans_space_with_format() {
        assert_eq!(
            parse_stream("[<j>]^ab/xy<n>$[</j>]^cd$"),
            Ok((
                "",
                vec![
                    StreamUnit::Format(String::from("<j>")),
                    StreamUnit::LexicalUnit(vec![
                        SubLU {
                            ling_form: String::from("ab"),
                            tags: vec![]
                        },
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")]
                        }
                    ]),
                    StreamUnit::Format(String::from("</j>")),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![]
                    }])
                ]
            ))
        );
    }

    #[test]
    fn parse_joined_lu_basic() {
        assert_eq!(
            parse_stream_unit("^ab/xy<n>+tx<a>$"),
            Ok((
                "",
                StreamUnit::JoinedLexicalUnit(vec![
                    vec![SubLU {
                        ling_form: String::from("ab"),
                        tags: vec![]
                    }],
                    vec![
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")]
                        },
                        SubLU {
                            ling_form: String::from("tx"),
                            tags: vec![String::from("a")]
                        }
                    ],
                ]),
            ))
        );
    }

    #[test]
    fn parse_chunk_with_format() {
        assert_eq!(
            parse_stream_unit("N1<SN><a>{^i$ [<o>]^j$[</o>]^k$}"),
            Ok((
                "",
                StreamUnit::Chunk(
                    SubLU {
                        ling_form: String::from("N1"),
                        tags: vec![String::from("SN"), String::from("a")]
                    },
                    vec![
                        StreamUnit::LexicalUnit(vec![SubLU {
                            ling_form: String::from("i"),
                            tags: vec![]
                        }]),
                        StreamUnit::Space(String::from(" ")),
                        StreamUnit::Format(String::from("<o>")),
                        StreamUnit::LexicalUnit(vec![SubLU {
                            ling_form: String::from("j"),
                            tags: vec![]
                        }]),
                        StreamUnit::Format(String::from("</o>")),
                        StreamUnit::LexicalUnit(vec![SubLU {
                            ling_form: String::from("k"),
                            tags: vec![]
                        }]),
                    ],
                ),
            ))
        );
    }
}
