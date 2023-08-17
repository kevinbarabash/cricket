use generational_arena::Arena;

use escalier_ast::*;
use escalier_parser::parse;

use escalier_hm::checker::Checker;
use escalier_hm::context::*;
use escalier_hm::errors::*;
use escalier_hm::types::{self, *};

fn test_env() -> (Checker, Context) {
    let mut checker = Checker {
        arena: Arena::new(),
    };
    let mut context = Context::default();

    let number = new_primitive(&mut checker.arena, Primitive::Number);
    let type_param_t = new_constructor(&mut checker.arena, "T", &[]);

    let push_t = checker.new_func_type(
        &[types::FuncParam {
            pattern: TPat::Ident(BindingIdent {
                name: "item".to_string(),
                mutable: false,
                span: Span { start: 0, end: 0 },
            }),
            t: type_param_t,
            optional: false,
        }],
        number,
        &None,
        None,
    );

    // [P]: T for P in number;
    let mapped = types::TObjElem::Mapped(types::MappedType {
        key: new_constructor(&mut checker.arena, "P", &[]),
        value: new_constructor(&mut checker.arena, "T", &[]),
        target: "P".to_string(),
        source: new_primitive(&mut checker.arena, Primitive::Number),
        check: None,
        extends: None,
    });

    let array_interface = new_object_type(
        &mut checker.arena,
        &[
            // .push(item: T) -> number;
            types::TObjElem::Prop(types::TProp {
                name: types::TPropKey::StringKey("push".to_string()),
                modifier: None,
                t: push_t,
                optional: false,
                mutable: false,
            }),
            // .length: number;
            types::TObjElem::Prop(types::TProp {
                name: types::TPropKey::StringKey("length".to_string()),
                modifier: None,
                optional: false,
                mutable: false,
                t: number,
            }),
            mapped,
        ],
    );
    let array_scheme = Scheme {
        type_params: Some(vec![types::TypeParam {
            name: "T".to_string(),
            constraint: None,
            default: None,
        }]),
        t: array_interface,
    };

    context.schemes.insert("Array".to_string(), array_scheme);

    (checker, context)
}

#[test]
fn basic_throws_test() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let add: fn (a: number, b: number) -> number
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) {
        return div(a, b)
    }
    let bar = fn (a, b) {
        return add(a, b)
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("div").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number throws "DIV_BY_ZERO""#
    );

    let binding = my_ctx.values.get("foo").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number throws "DIV_BY_ZERO""#
    );

    let binding = my_ctx.values.get("bar").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number"#
    );

    Ok(())
}

#[test]
fn constrained_throws() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) throws string {
        return div(a, b)
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("foo").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number throws string"#
    );

    Ok(())
}

#[test]
fn constrained_throws_type_mismatch() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) throws number {
        return div(a, b)
    }
    "#;
    let mut program = parse(src).unwrap();

    let result = checker.infer_program(&mut program, &mut my_ctx);

    assert_eq!(
        result,
        Err(Errors::InferenceError(
            "type mismatch: unify(\"DIV_BY_ZERO\", number) failed".to_string()
        ))
    );

    Ok(())
}

#[test]
fn throws_multiple_exceptions() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let sqrt: fn (a: number) -> number throws "NEGATIVE_NUMBER"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) {
        return sqrt(div(a, b))
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("foo").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number throws "NEGATIVE_NUMBER" | "DIV_BY_ZERO""#
    );

    Ok(())
}

#[test]
fn unify_call_throws_with_func_sig_throws() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let sqrt: fn (a: number) -> number throws "NEGATIVE_NUMBER"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) throws string {
        return sqrt(div(a, b))
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("foo").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number throws string"#
    );

    Ok(())
}

#[test]
fn unify_call_throws_with_func_sig_throws_failure() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let sqrt: fn (a: number) -> number throws "NEGATIVE_NUMBER"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) throws number {
        return sqrt(div(a, b))
    }
    "#;
    let mut program = parse(src).unwrap();

    let result = checker.infer_program(&mut program, &mut my_ctx);

    assert_eq!(
        result,
        Err(Errors::InferenceError(
            "type mismatch: unify(\"NEGATIVE_NUMBER\", number) failed".to_string()
        ))
    );

    Ok(())
}

#[test]
fn scoped_throws() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) {
        let inner = fn () => div(a, b)
        return a + b
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("foo").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number"#
    );

    Ok(())
}

#[test]
fn throws_coalesces_duplicate_exceptions() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    let foo = fn (a, b) {
        return div(1, a) + div(1, b)
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("foo").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        // TODO: dedupe this
        r#"(a: number, b: number) -> number throws "DIV_BY_ZERO""#
    );

    Ok(())
}

#[test]
fn callback_with_throws() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let div: fn (a: number, b: number) -> number throws "DIV_BY_ZERO"
    declare let map: fn<T, U, E>(
        elems: Array<T>,
        callback: fn(elem: T, index: number) -> U throws E,
    ) -> Array<U> throws E

    let array: Array<number> = [1, 2, 3]
    let foo = fn () {
        let result = map(array, fn (elem, index) => div(elem, index))
        return result
    }
    let bar = fn () {
        let result = map(array, fn (elem, index) => elem * elem)
        return result
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("foo").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"() -> Array<number> throws "DIV_BY_ZERO""#
    );

    let binding = my_ctx.values.get("bar").unwrap();
    assert_eq!(checker.print_type(&binding.index), r#"() -> Array<number>"#);

    Ok(())
}

#[test]
fn infer_throws_from_throw() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    let div = fn (a, b) {
        if (b == 0) {
            throw "DIV_BY_ZERO"
        }
        return a / b
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("div").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number throws "DIV_BY_ZERO""#
    );

    Ok(())
}

#[test]
fn try_catches_throw() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let log: fn (msg: string) -> undefined
    let div = fn (a, b) => try {
        if (b == 0) {
            throw "DIV_BY_ZERO"
        }
        a / b
    } catch (e) {
        log(e)
        0
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("div").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number | 0"#
    );

    Ok(())
}

#[test]
fn try_catches_throw_and_rethrows() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let log: fn (msg: string) -> undefined
    let div = fn (a, b) => try {
        if (b == 0) {
            throw "DIV_BY_ZERO"
        }
        a / b
    } catch (e) {
        log(e)
        throw "RETHROWN_ERROR"
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("div").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        r#"(a: number, b: number) -> number throws "RETHROWN_ERROR""#
    );

    Ok(())
}

#[test]
fn try_catches_throw_return_inside_try_catch() -> Result<(), Errors> {
    let (mut checker, mut my_ctx) = test_env();

    let src = r#"
    declare let log: fn (msg: string) -> undefined
    let div = fn (a, b) {
        try {
            if (b == 0) {
                throw "DIV_BY_ZERO"
            }
            return a / b
        } catch (e) {
            log(e)
            return 0
        }
    }
    "#;
    let mut program = parse(src).unwrap();

    checker.infer_program(&mut program, &mut my_ctx)?;

    let binding = my_ctx.values.get("div").unwrap();
    assert_eq!(
        checker.print_type(&binding.index),
        // TODO: the return type should be `number` because all `return` statements
        // return numbers and code appearing after the `try-catch` is
        // unreachable.
        r#"(a: number, b: number) -> undefined"#
    );

    Ok(())
}