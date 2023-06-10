use crate::expr::{BinaryOp, Expr, ExprKind, Literal, UnaryOp};
use crate::parser::Parser;
use crate::precedence::{Associativity, Operator, PRECEDENCE_TABLE};
use crate::source_location::*;
use crate::stmt::Stmt;
use crate::stmt_parser::parse_stmt;
use crate::token::{Token, TokenKind};

fn get_prefix_precedence(op: &Token) -> Option<(u8, Associativity)> {
    match &op.kind {
        TokenKind::Plus => PRECEDENCE_TABLE.get(&Operator::UnaryPlus).cloned(),
        TokenKind::Minus => PRECEDENCE_TABLE.get(&Operator::UnaryMinus).cloned(),
        _ => None,
    }
}

fn get_infix_precedence(op: &Token) -> Option<(u8, Associativity)> {
    match &op.kind {
        // multiplicative
        TokenKind::Times => PRECEDENCE_TABLE.get(&Operator::Multiplication).cloned(),
        TokenKind::Divide => PRECEDENCE_TABLE.get(&Operator::Division).cloned(),
        TokenKind::Modulo => PRECEDENCE_TABLE.get(&Operator::Remainder).cloned(),

        // additive
        TokenKind::Plus => PRECEDENCE_TABLE.get(&Operator::Addition).cloned(),
        TokenKind::Minus => PRECEDENCE_TABLE.get(&Operator::Subtraction).cloned(),

        // equality
        TokenKind::Equals => PRECEDENCE_TABLE.get(&Operator::Equals).cloned(),
        TokenKind::NotEquals => PRECEDENCE_TABLE.get(&Operator::NotEquals).cloned(),
        TokenKind::LessThan => PRECEDENCE_TABLE.get(&Operator::LessThan).cloned(),
        TokenKind::LessThanOrEqual => PRECEDENCE_TABLE.get(&Operator::LessThanOrEqual).cloned(),
        TokenKind::GreaterThan => PRECEDENCE_TABLE.get(&Operator::GreaterThan).cloned(),
        TokenKind::GreaterThanOrEqual => {
            PRECEDENCE_TABLE.get(&Operator::GreaterThanOrEqual).cloned()
        }

        // logic
        TokenKind::And => PRECEDENCE_TABLE.get(&Operator::LogicalAnd).cloned(),
        TokenKind::Or => PRECEDENCE_TABLE.get(&Operator::LogicalOr).cloned(),

        // assignment
        TokenKind::Assign => PRECEDENCE_TABLE.get(&Operator::Assignment).cloned(),
        _ => None,
    }
}

fn get_postfix_precedence(op: &Token) -> Option<(u8, Associativity)> {
    match &op.kind {
        TokenKind::LeftBracket => PRECEDENCE_TABLE
            .get(&Operator::ComputedMemberAccess)
            .cloned(),
        TokenKind::LeftParen => PRECEDENCE_TABLE.get(&Operator::FunctionCall).cloned(),
        // TODO: handle optional chaining
        TokenKind::Dot => PRECEDENCE_TABLE.get(&Operator::MemberAccess).cloned(),
        _ => None,
    }
}

fn parse_params(parser: &mut Parser) -> Vec<String> {
    let mut params = Vec::new();
    while parser.peek(0).kind != TokenKind::RightParen {
        let param = parser.next();
        if let TokenKind::Identifier(name) = param.kind {
            params.push(name.to_owned());
        } else {
            panic!("Expected identifier, got {:?}", param);
        }

        match parser.peek(0).kind {
            TokenKind::RightParen => break,
            TokenKind::Comma => {
                parser.next();
            }
            _ => panic!("Expected comma or right paren, got {:?}", parser.peek(0)),
        }
    }
    params
}

fn parse_block(parser: &mut Parser) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    while parser.peek(0).kind != TokenKind::RightBrace {
        stmts.push(parse_stmt(parser));
    }
    stmts
}

fn parse_expr_with_precedence(parser: &mut Parser, precedence: u8) -> Expr {
    let next = parser.next();

    let mut lhs = match &next.kind {
        // TODO:
        // - arrays
        // - objects
        // - do-expression
        // - if-else conditional
        // - try-catch
        // - match
        TokenKind::NumLit(n) => Expr {
            kind: ExprKind::Literal(Literal::Number(n.to_owned())),
            loc: next.loc.clone(),
        },
        TokenKind::Identifier(id) => Expr {
            kind: ExprKind::Identifier(id.to_owned()),
            loc: next.loc.clone(),
        },
        TokenKind::BoolLit(b) => Expr {
            kind: ExprKind::Literal(Literal::Boolean(*b)),
            loc: next.loc.clone(),
        },
        TokenKind::StrLit(s) => Expr {
            kind: ExprKind::Literal(Literal::String(s.to_owned())),
            loc: next.loc.clone(),
        },
        TokenKind::StrTemplateLit { parts, exprs } => Expr {
            kind: ExprKind::TemplateLiteral {
                parts: parts
                    .iter()
                    .map(|s| match &s.kind {
                        TokenKind::StrLit(s) => Literal::String(s.to_owned()),
                        _ => panic!("Expected string literal, got {:?}", s),
                    })
                    .collect(),
                exprs: exprs.to_owned(),
            },
            loc: next.loc.clone(),
        },
        TokenKind::Null => Expr {
            kind: ExprKind::Literal(Literal::Null),
            loc: next.loc.clone(),
        },
        TokenKind::Undefined => Expr {
            kind: ExprKind::Literal(Literal::Null),
            loc: next.loc.clone(),
        },
        TokenKind::LeftParen => {
            let lhs = parse_expr_with_precedence(parser, 0);
            assert_eq!(parser.next().kind, TokenKind::RightParen);
            lhs
        }
        TokenKind::Fn => {
            assert_eq!(parser.next().kind, TokenKind::LeftParen);
            let params = parse_params(parser);
            assert_eq!(parser.next().kind, TokenKind::RightParen);

            match parser.next().kind {
                TokenKind::LeftBrace => {
                    let body = parse_block(parser);
                    let close_brace = parser.next();
                    assert_eq!(close_brace.kind, TokenKind::RightBrace);

                    let loc = merge_locations(&next.loc, &close_brace.loc);
                    Expr {
                        kind: ExprKind::Function { params, body },
                        loc,
                    }
                }
                TokenKind::Arrow => {
                    let expr = parse_expr(parser);
                    let loc = merge_locations(&next.loc, &expr.loc);

                    Expr {
                        kind: ExprKind::Lambda {
                            params,
                            expr: Box::new(expr),
                        },
                        loc,
                    }
                }
                _ => panic!("Expected left brace or arrow, got {:?}", parser.peek(0)),
            }
        }
        TokenKind::If => {
            assert_eq!(parser.next().kind, TokenKind::LeftParen);
            let cond = parse_expr(parser);
            assert_eq!(parser.next().kind, TokenKind::RightParen);
            assert_eq!(parser.next().kind, TokenKind::LeftBrace);
            let consequent = parse_block(parser);
            let close_brace = parser.next();
            assert_eq!(close_brace.kind, TokenKind::RightBrace);

            if parser.peek(0).kind == TokenKind::Else {
                parser.next();
                assert_eq!(parser.next().kind, TokenKind::LeftBrace);
                let alternate = parse_block(parser);
                let close_brace = parser.next();
                assert_eq!(close_brace.kind, TokenKind::RightBrace);

                let loc = merge_locations(&next.loc, &close_brace.loc);
                Expr {
                    kind: ExprKind::IfElse {
                        cond: Box::new(cond),
                        consequent,
                        alternate: Some(alternate),
                    },
                    loc,
                }
            } else {
                let loc = merge_locations(&next.loc, &close_brace.loc);
                Expr {
                    kind: ExprKind::IfElse {
                        cond: Box::new(cond),
                        consequent,
                        alternate: None,
                    },
                    loc,
                }
            }
        }
        t => match get_prefix_precedence(&next) {
            Some(precendence) => {
                let op = match t {
                    TokenKind::Plus => UnaryOp::Plus,
                    TokenKind::Minus => UnaryOp::Minus,
                    _ => panic!("unexpected token: {:?}", t),
                };

                let rhs = parse_expr_with_precedence(parser, precendence.0);
                let loc = merge_locations(&next.loc, &rhs.loc);
                Expr {
                    kind: ExprKind::Unary {
                        op,
                        right: Box::new(rhs),
                    },
                    loc,
                }
            }
            None => panic!("unexpected token: {:?}", next),
        },
    };

    loop {
        let next = parser.peek(0);
        if let TokenKind::Eof = next.kind {
            break;
        }

        if let TokenKind::Semicolon = next.kind {
            break;
        }

        if let Some(next_precedence) = get_postfix_precedence(&next) {
            if precedence >= next_precedence.0 {
                break;
            }

            let precedence = if next_precedence.1 == Associativity::Left {
                next_precedence.0
            } else {
                next_precedence.0 - 1
            };

            parser.next();

            lhs = match &next.kind {
                TokenKind::LeftBracket => {
                    let rhs = parse_expr(parser);
                    let loc = merge_locations(&lhs.loc, &rhs.loc);
                    assert_eq!(parser.next().kind, TokenKind::RightBracket);
                    Expr {
                        kind: ExprKind::Index {
                            left: Box::new(lhs),
                            right: Box::new(rhs),
                        },
                        loc,
                    }
                }
                TokenKind::LeftParen => {
                    let mut args = Vec::new();
                    while parser.peek(0).kind != TokenKind::RightParen {
                        args.push(parse_expr(parser));

                        match parser.peek(0).kind {
                            TokenKind::RightParen => break,
                            TokenKind::Comma => {
                                parser.next();
                            }
                            _ => panic!("Expected comma or right paren, got {:?}", parser.peek(0)),
                        }
                    }
                    let loc = merge_locations(&lhs.loc, &parser.peek(0).loc);
                    assert_eq!(parser.next().kind, TokenKind::RightParen);
                    Expr {
                        kind: ExprKind::Call {
                            callee: Box::new(lhs),
                            args,
                        },
                        loc,
                    }
                }
                TokenKind::Dot => {
                    let rhs = parse_expr_with_precedence(parser, precedence);
                    let loc = merge_locations(&lhs.loc, &rhs.loc);
                    Expr {
                        kind: ExprKind::Member {
                            object: Box::new(lhs),
                            property: Box::new(rhs),
                        },
                        loc,
                    }
                }
                _ => panic!("unexpected token: {:?}", next),
            };

            continue;
        }

        if let Some(next_precedence) = get_infix_precedence(&next) {
            if precedence >= next_precedence.0 {
                break;
            }

            parser.next();

            let op: BinaryOp = match &next.kind {
                TokenKind::Plus => BinaryOp::Plus,
                TokenKind::Minus => BinaryOp::Minus,
                TokenKind::Times => BinaryOp::Times,
                TokenKind::Divide => BinaryOp::Divide,
                TokenKind::Modulo => BinaryOp::Modulo,
                TokenKind::Equals => BinaryOp::Equals,
                TokenKind::NotEquals => BinaryOp::NotEquals,
                TokenKind::LessThan => BinaryOp::LessThan,
                TokenKind::LessThanOrEqual => BinaryOp::LessThanOrEqual,
                TokenKind::GreaterThan => BinaryOp::GreaterThan,
                TokenKind::GreaterThanOrEqual => BinaryOp::GreaterThanOrEqual,
                TokenKind::And => BinaryOp::And,
                TokenKind::Or => BinaryOp::Or,
                _ => panic!("unexpected token: {:?}", next),
            };

            let precedence = if next_precedence.1 == Associativity::Left {
                next_precedence.0
            } else {
                next_precedence.0 - 1
            };

            let rhs = parse_expr_with_precedence(parser, precedence);
            let loc = merge_locations(&lhs.loc, &rhs.loc);

            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                loc,
            };

            continue;
        }

        break;
    }

    lhs
}

pub fn parse_expr(parser: &mut Parser) -> Expr {
    parse_expr_with_precedence(parser, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    pub fn parse(input: &str) -> Expr {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.lex();
        let mut parser = Parser::new(tokens);
        parse_expr(&mut parser)
    }

    #[test]
    fn parse_literals() {
        insta::assert_debug_snapshot!(parse("123"));
        insta::assert_debug_snapshot!(parse("true"));
        insta::assert_debug_snapshot!(parse("false"));
        insta::assert_debug_snapshot!(parse("null"));
        insta::assert_debug_snapshot!(parse("undefined"));
        insta::assert_debug_snapshot!(parse(r#""hello""#));
    }

    #[test]
    fn parse_simple_addition() {
        insta::assert_debug_snapshot!(parse("1 + 2 + 3"));
    }

    #[test]
    fn parse_addition_and_subtraction() {
        insta::assert_debug_snapshot!(parse("1 + 2 - 3 + 4"));
    }

    #[test]
    fn parse_multiplicative_operators() {
        insta::assert_debug_snapshot!(parse("a * b / c % d"));
    }

    #[test]
    fn parse_additive_and_multiplicative() {
        insta::assert_debug_snapshot!(parse("1 * 2 + 3"));
    }

    #[test]
    fn parse_parens() {
        insta::assert_debug_snapshot!(parse("5 * (x + 1)"));
    }

    #[test]
    fn parse_comparisons_and_logic() {
        insta::assert_debug_snapshot!(parse("a > b && c >= d || e < f && g <= h"));
        insta::assert_debug_snapshot!(parse("x != y && z == w"));
    }

    #[test]
    fn parse_unary_operators() {
        insta::assert_debug_snapshot!(parse("--a - +b"));
    }

    #[test]
    fn parse_indexing() {
        insta::assert_debug_snapshot!(parse("a[1][c]"));
    }

    #[test]
    fn parse_function() {
        insta::assert_debug_snapshot!(parse("fn () { let x = 5; let y = 10; return x + y; }"));
    }

    #[test]
    fn parse_function_with_params() {
        let src = r#"fn (x, y) { return x + y; }"#;
        insta::assert_debug_snapshot!(parse(src));
    }

    #[test]
    fn parse_lambdas() {
        insta::assert_debug_snapshot!(parse("fn (x, y) { return x + y; }"));
        insta::assert_debug_snapshot!(parse("fn (x) => fn (y) => x + y;"));
    }

    #[test]
    #[should_panic]
    fn parse_function_expected_comma_or_left_brace() {
        let src = r#"fn (x, y { return x + y; }"#;
        parse(src);
    }

    #[test]
    #[should_panic]
    fn parse_function_expected_identifier() {
        let src = r#"fn (, y) { return x + y; }"#;
        parse(src);
    }

    #[test]
    fn parse_function_call() {
        insta::assert_debug_snapshot!(parse("add(5, 10)"));
        insta::assert_debug_snapshot!(parse("add(5)(10)"));
    }

    #[test]
    fn parse_call_expr() {
        let src = r#"foo[bar](5, 10)"#;
        insta::assert_debug_snapshot!(parse(src));
    }

    #[test]
    fn parse_member_access() {
        insta::assert_debug_snapshot!(parse("a.b.c"));
        insta::assert_debug_snapshot!(parse("a.b+c.d"));
        insta::assert_debug_snapshot!(parse("a[b][c]"));
    }

    #[test]
    fn parse_callback() {
        insta::assert_debug_snapshot!(parse(r#"ids.map(fn (id) => id).join(", ")"#));
    }

    #[test]
    fn parse_conditionals() {
        insta::assert_debug_snapshot!(parse(r#"if (cond) { x; }"#));
        insta::assert_debug_snapshot!(parse(r#"if (cond) { x; } else { y; }"#));
        // TODO: enable this test case once we have object literals
        // insta::assert_debug_snapshot!(parse(
        //     r#"
        //     if (cond) {
        //         {x: 5, y: 10};
        //     } else {
        //         {a: 1, b: 2};
        //     }
        //     "#
        // ));
    }
}