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
use nom::sequence::tuple;
use nom::IResult;

#[derive(Debug, PartialEq)]
pub enum Flag {
    Nothing,
    Unanalyzed,
    Untranslated,
    UnableToGenerateOrStartOfInvariablePart,
}

#[derive(Debug, PartialEq)]
pub struct SubLU {
    ling_form: String,
    flag: Flag,
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

pub fn make_flag(s: &str) -> Flag {
    match s {
        "*" => Flag::Unanalyzed,
        "@" => Flag::Untranslated,
        "#" => Flag::UnableToGenerateOrStartOfInvariablePart,
        _ => Flag::Nothing,
    }
}

pub fn parse_sub_lu_basic(input: &str) -> IResult<&str, SubLU> {
    let ling_form_inner_parse = is_not(r#"^$@*/<>{}\[]"#);
    let ling_form_escape_parse =
        escaped_transform(ling_form_inner_parse, '\\', one_of(r#"^$@*/<>{}\[]"#));
    let mut parse = tuple((
        alt((tag("*"), tag("#"), tag("@"), tag(""))),
        ling_form_escape_parse,
        many0(parse_tag),
    ));
    parse(input).map(|(i, (flag, ling_form, tags))| {
        (
            i,
            SubLU {
                ling_form: ling_form.to_string(),
                tags: tags.iter().map(|tag| String::from(*tag)).collect(),
                flag: make_flag(flag),
            },
        )
    })
}

pub fn parse_sub_lu_without_ling_form(input: &str) -> IResult<&str, SubLU> {
    let mut parse = tuple((
        alt((tag("*"), tag("#"), tag("@"), tag(""))),
        many0(parse_tag),
    ));
    parse(input).map(|(i, (flag, tags))| {
        (
            i,
            SubLU {
                ling_form: String::from(""),
                tags: tags.iter().map(|tag| String::from(*tag)).collect(),
                flag: make_flag(flag),
            },
        )
    })
}

pub fn parse_sub_lu(input: &str) -> IResult<&str, SubLU> {
    alt((parse_sub_lu_basic, parse_sub_lu_without_ling_form))(input)
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
    alt((space1, tag("\n")))(input).map(|(i, o)| (i, StreamUnit::Space(String::from(o))))
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
    use slurp;

    #[test]
    fn basic_lu() {
        assert_eq!(
            parse_stream_unit("^กา$"),
            Ok((
                "",
                StreamUnit::LexicalUnit(vec![SubLU {
                    ling_form: String::from("กา"),
                    tags: vec![],
                    flag: Flag::Nothing,
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
                    tags: vec![],
                    flag: Flag::Nothing,
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
                        tags: vec![],
                        flag: Flag::Nothing,
                    },
                    SubLU {
                        ling_form: String::from("xy"),
                        tags: vec![],
                        flag: Flag::Nothing,
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
                    tags: vec![],
                    flag: Flag::Nothing,
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
                        tags: vec![],
                        flag: Flag::Nothing,
                    }]),
                    StreamUnit::Space(String::from(" ")),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![],
                        flag: Flag::Nothing,
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
                            tags: vec![],
                            flag: Flag::Nothing,
                        },
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")],
                            flag: Flag::Nothing,
                        }
                    ]),
                    StreamUnit::Space(String::from(" ")),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![],
                        flag: Flag::Nothing,
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
                            tags: vec![],
                            flag: Flag::Nothing,
                        },
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")],
                            flag: Flag::Nothing,
                        }
                    ]),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![],
                        flag: Flag::Nothing,
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
                            tags: vec![],
                            flag: Flag::Nothing,
                        },
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")],
                            flag: Flag::Nothing,
                        }
                    ]),
                    StreamUnit::Format(String::from("</j>")),
                    StreamUnit::LexicalUnit(vec![SubLU {
                        ling_form: String::from("cd"),
                        tags: vec![],
                        flag: Flag::Nothing,
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
                        tags: vec![],
                        flag: Flag::Nothing,
                    }],
                    vec![
                        SubLU {
                            ling_form: String::from("xy"),
                            tags: vec![String::from("n")],
                            flag: Flag::Nothing,
                        },
                        SubLU {
                            ling_form: String::from("tx"),
                            tags: vec![String::from("a")],
                            flag: Flag::Nothing,
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
                        tags: vec![String::from("SN"), String::from("a")],
                        flag: Flag::Nothing,
                    },
                    vec![
                        StreamUnit::LexicalUnit(vec![SubLU {
                            ling_form: String::from("i"),
                            tags: vec![],
                            flag: Flag::Nothing,
                        }]),
                        StreamUnit::Space(String::from(" ")),
                        StreamUnit::Format(String::from("<o>")),
                        StreamUnit::LexicalUnit(vec![SubLU {
                            ling_form: String::from("j"),
                            tags: vec![],
                            flag: Flag::Nothing,
                        }]),
                        StreamUnit::Format(String::from("</o>")),
                        StreamUnit::LexicalUnit(vec![SubLU {
                            ling_form: String::from("k"),
                            tags: vec![],
                            flag: Flag::Nothing,
                        }]),
                    ],
                ),
            ))
        );
    }

    #[test]
    fn parse_escape_bracket() {
        let raw = "^\\]<vblex><pres>$";
        let (i, _) = parse_stream_unit(raw).unwrap();
        assert_eq!(i.len(), 0);
    }

    #[test]
    fn parse_at_lu() {
        let raw = "^\\@<det><ind><sg>$";
        let (i, _) = parse_stream_unit(raw).unwrap();
        assert_eq!(i.len(), 0);
    }

    #[test]
    fn parse_special_lemma() {
        let raw = "^*t<det><ind><sg>$";
        let (i, su) = parse_stream_unit(&raw).unwrap();
        assert_eq!(i.len(), 0);
        assert_eq!(
            su,
            StreamUnit::LexicalUnit(vec![SubLU {
                ling_form: String::from("t"),
                tags: vec![String::from("det"), String::from("ind"), String::from("sg")],
                flag: Flag::Unanalyzed,
            }])
        )
    }

    #[test]
    fn parse_special_lemma_only() {
        let raw = "^*<det><ind><sg>$";
        let (i, su) = parse_stream_unit(&raw).unwrap();
        assert_eq!(i.len(), 0);
        assert_eq!(
            su,
            StreamUnit::LexicalUnit(vec![SubLU {
                ling_form: String::from(""),
                tags: vec![String::from("det"), String::from("ind"), String::from("sg")],
                flag: Flag::Unanalyzed,
            }])
        )
    }

    #[test]
    fn parse_large_thai_data() {
        let raw = slurp::read_all_to_string("test_data/i_like_a_dog_sent.apertium_stream").unwrap();
        let (i, stream) = parse_stream(&raw).unwrap();
        assert_eq!(i.len(), 0);
        assert_eq!(
            stream
                .into_iter()
                .filter_map(|su| match su {
                    StreamUnit::Space(_) => None,
                    StreamUnit::LexicalUnit(analyses) => Some(analyses[0].ling_form.clone()),
                    _ => Some(String::from("_")),
                })
                .count(),
            5
        );
    }
}
