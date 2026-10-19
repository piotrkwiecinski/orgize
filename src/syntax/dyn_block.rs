use nom::Parser;
use nom::{
    bytes::complete::{tag_no_case, take_till1},
    character::complete::{space0, space1},
    IResult, Input as _,
};

use super::{
    combinator::{
        blank_lines, eol_or_eof, line_starts_iter, node, trim_line_end, GreenElement, NodeBuilder,
    },
    input::Input,
    SyntaxKind::*,
};

fn dyn_block_node_base(input: Input) -> IResult<Input, GreenElement, ()> {
    let (input, begin) = dyn_block_begin_node(input)?;
    let (input, pre_blank) = blank_lines(input)?;

    for (input, contents) in line_starts_iter(input.as_str()).map(|i| input.take_split(i)) {
        if let Ok((input, end)) = dyn_block_end_node(input) {
            let (input, post_blank) = blank_lines(input)?;
            let mut children = vec![begin];
            children.extend(pre_blank);
            children.push(contents.text_token());
            children.push(end);
            children.extend(post_blank);

            return Ok((input, node(DYN_BLOCK, children)));
        }
    }

    Err(nom::Err::Error(()))
}

fn dyn_block_begin_node(input: Input) -> IResult<Input, GreenElement, ()> {
    let (input, (ws, begin, ws_, name, (args, ws__, nl))) = (
        space0,
        tag_no_case("#+BEGIN:"),
        space1,
        take_till1(|c: char| c.is_ascii_whitespace()),
        trim_line_end,
    )
        .parse(input)?;

    let mut b = NodeBuilder::new();
    b.ws(ws);
    b.text(begin);
    b.ws(ws_);
    b.text(name);
    b.text(args);
    b.ws(ws__);
    b.nl(nl);

    Ok((input, b.finish(DYN_BLOCK_BEGIN)))
}

fn dyn_block_end_node(input: Input) -> IResult<Input, GreenElement, ()> {
    let (input, (ws, end, ws_, nl)) =
        (space0, tag_no_case("#+END:"), space0, eol_or_eof).parse(input)?;

    let mut b = NodeBuilder::new();
    b.ws(ws);
    b.text(end);
    b.ws(ws_);
    b.nl(nl);

    Ok((input, b.finish(DYN_BLOCK_END)))
}

#[cfg_attr(
    feature = "tracing",
    tracing::instrument(level = "debug", skip(input), fields(input = input.s))
)]
pub fn dyn_block_node(input: Input) -> IResult<Input, GreenElement, ()> {
    crate::lossless_parser!(dyn_block_node_base, input)
}

#[test]
fn parse() {
    use crate::{ast::DynBlock, tests::to_ast};

    let to_dyn_block = to_ast::<DynBlock>(dyn_block_node);

    insta::assert_debug_snapshot!(
        to_dyn_block(
            r#"#+BEGIN: clocktable :scope file

CONTENTS
#+END:
    "#).syntax,
        @r###"
    DYN_BLOCK@0..53
      DYN_BLOCK_BEGIN@0..32
        TEXT@0..8 "#+BEGIN:"
        WHITESPACE@8..9 " "
        TEXT@9..19 "clocktable"
        TEXT@19..31 " :scope file"
        NEW_LINE@31..32 "\n"
      BLANK_LINE@32..33 "\n"
      TEXT@33..42 "CONTENTS\n"
      DYN_BLOCK_END@42..49
        TEXT@42..48 "#+END:"
        NEW_LINE@48..49 "\n"
      BLANK_LINE@49..53 "    "
    "###
    );
}

#[test]
fn dyn_block_element() {
    use crate::syntax::{combinator::node, element::element_nodes, SyntaxKind, SyntaxNode};
    use crate::ParseConfig;

    let t = |input: &str| {
        let config = &ParseConfig::default();
        let children = element_nodes((input, config).into()).unwrap();
        SyntaxNode::new_root(node(SyntaxKind::SECTION, children).into_node().unwrap())
    };

    // a dynamic block, not a `#+BEGIN:` keyword, with any name and with
    // affiliated keywords
    insta::assert_debug_snapshot!(
        t("#+name: d\n#+BEGIN: clock-table :scope file\n| a |\n#+END:\n"),
        @r##"
    SECTION@0..56
      DYN_BLOCK@0..56
        AFFILIATED_KEYWORD@0..10
          HASH_PLUS@0..2 "#+"
          TEXT@2..6 "name"
          COLON@6..7 ":"
          TEXT@7..9 " d"
          NEW_LINE@9..10 "\n"
        DYN_BLOCK_BEGIN@10..43
          TEXT@10..18 "#+BEGIN:"
          WHITESPACE@18..19 " "
          TEXT@19..30 "clock-table"
          TEXT@30..42 " :scope file"
          NEW_LINE@42..43 "\n"
        TEXT@43..49 "| a |\n"
        DYN_BLOCK_END@49..56
          TEXT@49..55 "#+END:"
          NEW_LINE@55..56 "\n"
    "##
    );
}
