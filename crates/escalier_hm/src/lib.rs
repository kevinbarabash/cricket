// Based on https://github.com/tcr/rust-hindley-milner/blob/master/src/lib.rs
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate lazy_static;

mod syntax;
mod types;

use std::collections::{HashMap, HashSet};

use crate::syntax::*;
use crate::types::*;

pub enum Errors {
    InferenceError(String),
    ParseError(String),
}

impl Namer {
    fn next(&mut self) -> String {
        let v = self.value;
        self.value = ((self.value as u8) + 1) as char;
        format!("{}", v)
    }

    fn name(&mut self, t: ArenaType) -> String {
        let k = self.set.get(&t).cloned();
        if let Some(val) = k {
            val
        } else {
            let v = self.next();
            self.set.insert(t, v.clone());
            v
        }
    }
}

/// A binary type constructor which builds function types
pub fn new_function(a: &mut Vec<Type>, params: &[ArenaType], ret: ArenaType) -> ArenaType {
    let t = Type::new_function(a.len(), params, ret);
    a.push(t);
    a.len() - 1
}

/// A binary type constructor which builds function types
pub fn new_variable(a: &mut Vec<Type>) -> ArenaType {
    let t = Type::new_variable(a.len());
    a.push(t);
    a.len() - 1
}

/// A binary type constructor which builds function types
pub fn new_operator(a: &mut Vec<Type>, name: &str, types: &[ArenaType]) -> ArenaType {
    let t = Type::new_operator(a.len(), name, types);
    a.push(t);
    a.len() - 1
}

// Basic types are constructed with a nullary type constructor
lazy_static! {
    // Basic integer
    static ref INTEGER: Type = Type::new_operator(0, "int", &[]);
    // Basic bool
    static ref BOOL: Type = Type::new_operator(1, "bool", &[]);
}

// Type inference machinery

#[derive(Clone, Debug)]
pub struct Env(HashMap<String, ArenaType>);

/// Computes the type of the expression given by node.
///
/// The type of the node is computed in the context of the
/// supplied type environment env. Data types can be introduced into the
/// language simply by having a predefined set of identifiers in the initial
/// environment. environment; this way there is no need to change the syntax or, more
/// importantly, the type-checking program when extending the language.
///
/// Args:
///     node: The root of the abstract syntax tree.
///     env: The type environment is a mapping of expression identifier names
///         to type assignments.
///     non_generic: A set of non-generic variables, or None
///
/// Returns:
///     The computed type of the expression.
///
/// Raises:
///     InferenceError: The type of the expression could not be inferred, for example
///         if it is not possible to unify two types such as Integer and Bool
///     ParseError: The abstract syntax tree rooted at node could not be parsed
pub fn analyse(
    a: &mut Vec<Type>,
    node: &Syntax,
    env: &Env,
    non_generic: &HashSet<ArenaType>,
) -> ArenaType {
    match node {
        Syntax::Identifier(Identifier { name }) => get_type(a, name, env, non_generic),
        Syntax::Apply(Apply { func, args }) => {
            let fun_type = analyse(a, func, env, non_generic);
            let arg_types = args
                .iter()
                .map(|arg| analyse(a, arg, env, non_generic))
                .collect::<Vec<_>>();
            let result_type = new_variable(a);
            let first = new_function(a, &arg_types, result_type);
            unify(a, first, fun_type);
            result_type
        }
        Syntax::Lambda(Lambda { params, body }) => {
            let mut param_types = vec![];
            let mut new_env = env.clone();
            let mut new_non_generic = non_generic.clone();
            for param in params {
                let arg_type = new_variable(a);
                new_env.0.insert(param.clone(), arg_type);
                new_non_generic.insert(arg_type);
                param_types.push(arg_type);
            }
            let result_type = analyse(a, body, &new_env, &new_non_generic);
            new_function(a, &param_types, result_type)
        }
        Syntax::Let(Let { defn, v, body }) => {
            let defn_type = analyse(a, defn, env, non_generic);
            let mut new_env = env.clone();
            new_env.0.insert(v.clone(), defn_type);
            analyse(a, body, &new_env, non_generic)
        }
        Syntax::Letrec(Letrec { defn, v, body }) => {
            let new_type = new_variable(a);
            let mut new_env = env.clone();
            new_env.0.insert(v.clone(), new_type);
            let mut new_non_generic = non_generic.clone();
            new_non_generic.insert(new_type);
            let defn_type = analyse(a, defn, &new_env, &new_non_generic);
            unify(a, new_type, defn_type);
            analyse(a, body, &new_env, non_generic)
        }
    }
}

/// Get the type of identifier name from the type environment env.
///
/// Args:
///     name: The identifier name
///     env: The type environment mapping from identifier names to types
///     non_generic: A set of non-generic TypeVariables
///
/// Raises:
///     ParseError: Raised if name is an undefined symbol in the type
///         environment.
fn get_type(
    a: &mut Vec<Type>,
    name: &str,
    env: &Env,
    non_generic: &HashSet<ArenaType>,
) -> ArenaType {
    if let Some(value) = env.0.get(name) {
        let mat = non_generic.iter().cloned().collect::<Vec<_>>();
        fresh(a, *value, &mat)
    } else if is_integer_literal(name) {
        0 //INTEGER.id
    } else {
        //raise ParseError("Undefined symbol {0}".format(name))
        panic!("Undefined symbol {:?}", name);
    }
}

/// Makes a copy of a type expression.
///
/// The type t is copied. The the generic variables are duplicated and the
/// non_generic variables are shared.
///
/// Args:
///     t: A type to be copied.
///     non_generic: A set of non-generic TypeVariables
fn fresh(a: &mut Vec<Type>, t: ArenaType, non_generic: &[ArenaType]) -> ArenaType {
    // A mapping of TypeVariables to TypeVariables
    let mut mappings = hashmap![];

    fn freshrec(
        a: &mut Vec<Type>,
        tp: ArenaType,
        mappings: &mut HashMap<ArenaType, ArenaType>,
        non_generic: &[ArenaType],
    ) -> ArenaType {
        let p = prune(a, tp);
        // We clone here because we can't move out of a shared reference.
        // TODO: Consider using Rc<RefCell<Type>> to avoid unnecessary cloning.
        match &a.get(p).unwrap().clone().kind {
            TypeKind::Variable(_) => {
                if is_generic(a, p, non_generic) {
                    mappings
                        .entry(p)
                        .or_insert_with(|| new_variable(a))
                        .to_owned()
                } else {
                    p
                }
            }
            TypeKind::Constructor(con) => {
                let b = con
                    .types
                    .iter()
                    .map(|x| freshrec(a, *x, mappings, non_generic))
                    .collect::<Vec<_>>();
                new_operator(a, &con.name, &b)
            }
            TypeKind::Function(func) => {
                let params = func
                    .params
                    .iter()
                    .map(|x| freshrec(a, *x, mappings, non_generic))
                    .collect::<Vec<_>>();
                let ret = freshrec(a, func.ret, mappings, non_generic);
                new_function(a, &params, ret)
            }
        }
    }

    freshrec(a, t, &mut mappings, non_generic)
}

/// Unify the two types t1 and t2.
///
/// Makes the types t1 and t2 the same.
///
/// Args:
///     t1: The first type to be made equivalent
///     t2: The second type to be be equivalent
///
/// Returns:
///     None
///
/// Raises:
///     InferenceError: Raised if the types cannot be unified.
fn unify(alloc: &mut Vec<Type>, t1: ArenaType, t2: ArenaType) {
    let a = prune(alloc, t1);
    let b = prune(alloc, t2);
    // Why do we clone here?
    let a_t = alloc.get(a).unwrap().clone();
    let b_t = alloc.get(b).unwrap().clone();
    // eprintln!("unify {:?} {:?}", a_t, b_t);
    match (&a_t.kind, &b_t.kind) {
        (TypeKind::Variable(_), _) => bind(alloc, a, b),
        (_, TypeKind::Variable(_)) => bind(alloc, b, a),
        (TypeKind::Constructor(con_a), TypeKind::Constructor(con_b)) => {
            // TODO: support type constructors with optional and default type params
            if con_a.name != con_b.name || con_a.types.len() != con_b.types.len() {
                // raise InferenceError("Type mismatch: {0} != {1}".format(str(a), str(b)))
                panic!("type mismatch");
            }
            for (p, q) in con_a.types.iter().zip(con_b.types.iter()) {
                unify(alloc, *p, *q);
            }
        }
        (TypeKind::Function(func_a), TypeKind::Function(func_b)) => {
            for (p, q) in func_a.params.iter().zip(func_b.params.iter()) {
                unify(alloc, *p, *q);
            }
            unify(alloc, func_a.ret, func_b.ret);
        }
        (l, r) => {
            panic!("type mismatch: unify({l:?}, {r:?}) failed");
        }
    }
}

fn bind(alloc: &mut Vec<Type>, a: usize, b: usize) {
    if a != b {
        if occurs_in_type(alloc, a, b) {
            // raise InferenceError("recursive unification")
            panic!("recursive unification");
        }
        alloc.get_mut(a).unwrap().set_instance(b);
    }
}

/// Returns the currently defining instance of t.
///
/// As a side effect, collapses the list of type instances. The function Prune
/// is used whenever a type expression has to be inspected: it will always
/// return a type expression which is either an uninstantiated type variable or
/// a type operator; i.e. it will skip instantiated variables, and will
/// actually prune them from expressions to remove long chains of instantiated
/// variables.
///
/// Args:
///     t: The type to be pruned
///
/// Returns:
///     An uninstantiated TypeVariable or a TypeOperator
fn prune(a: &mut Vec<Type>, t: ArenaType) -> ArenaType {
    let v2 = match a.get(t).unwrap().kind {
        // TODO: handle .unwrap() panicing
        TypeKind::Variable(Variable {
            instance: Some(value),
        }) => value,
        _ => {
            return t;
        }
    };

    let value = prune(a, v2);
    match &mut a.get_mut(t).unwrap().kind {
        // TODO: handle .unwrap() panicing
        TypeKind::Variable(Variable {
            ref mut instance, ..
        }) => {
            *instance = Some(value);
        }
        _ => {
            return t;
        }
    }
    value
}

/// Checks whether a given variable occurs in a list of non-generic variables
///
/// Note that a variables in such a list may be instantiated to a type term,
/// in which case the variables contained in the type term are considered
/// non-generic.
///
/// Note: Must be called with v pre-pruned
///
/// Args:
///     v: The TypeVariable to be tested for genericity
///     non_generic: A set of non-generic TypeVariables
///
/// Returns:
///     True if v is a generic variable, otherwise False
fn is_generic(a: &mut Vec<Type>, v: ArenaType, non_generic: &[ArenaType]) -> bool {
    !occurs_in(a, v, non_generic)
}

/// Checks whether a type variable occurs in a type expression.
///
/// Note: Must be called with v pre-pruned
///
/// Args:
///     v:  The TypeVariable to be tested for
///     type2: The type in which to search
///
/// Returns:
///     True if v occurs in type2, otherwise False
fn occurs_in_type(a: &mut Vec<Type>, v: ArenaType, type2: ArenaType) -> bool {
    let pruned_type2 = prune(a, type2);
    if pruned_type2 == v {
        return true;
    }
    // We clone here because we can't move out of a shared reference.
    // TODO: Consider using Rc<RefCell<Type>> to avoid unnecessary cloning.
    match a.get(pruned_type2).unwrap().clone().kind {
        TypeKind::Constructor(Constructor { types, .. }) => occurs_in(a, v, &types),
        TypeKind::Function(Function { params, ret, .. }) => {
            occurs_in(a, v, &params) || occurs_in_type(a, v, ret)
        }
        _ => false,
    }
}

/// Checks whether a types variable occurs in any other types.
///
/// Args:
///     t:  The TypeVariable to be tested for
///     types: The sequence of types in which to search
///
/// Returns:
///     True if t occurs in any of types, otherwise False
///
fn occurs_in(a: &mut Vec<Type>, t: ArenaType, types: &[ArenaType]) -> bool {
    for t2 in types.iter() {
        if occurs_in_type(a, t, *t2) {
            return true;
        }
    }
    false
}

/// Checks whether name is an integer literal string.
///
/// Args:
///     name: The identifier to check
///
/// Returns:
///     True if name is an integer literal, otherwise False
fn is_integer_literal(name: &str) -> bool {
    name.parse::<isize>().is_ok()
}

//=====================================================

#[cfg(test)]
mod tests {
    use super::*;

    pub fn new_lambda(params: &[&str], body: Syntax) -> Syntax {
        Syntax::Lambda(Lambda {
            params: params.iter().map(|x| x.to_string()).collect(),
            body: Box::new(body),
        })
    }

    pub fn new_apply(func: Syntax, args: &[Syntax]) -> Syntax {
        Syntax::Apply(Apply {
            func: Box::new(func),
            args: args.to_owned(),
        })
    }

    pub fn new_let(v: &str, defn: Syntax, body: Syntax) -> Syntax {
        Syntax::Let(Let {
            v: v.to_string(),
            defn: Box::new(defn),
            body: Box::new(body),
        })
    }

    pub fn new_letrec(v: &str, defn: Syntax, body: Syntax) -> Syntax {
        Syntax::Letrec(Letrec {
            v: v.to_string(),
            defn: Box::new(defn),
            body: Box::new(body),
        })
    }

    pub fn new_identifier(name: &str) -> Syntax {
        Syntax::Identifier(Identifier {
            name: name.to_string(),
        })
    }

    fn test_env() -> (Vec<Type>, Env) {
        let mut a = vec![INTEGER.clone(), BOOL.clone()];
        let var1 = new_variable(&mut a);
        let var2 = new_variable(&mut a);
        let pair_type = new_operator(&mut a, "*", &[var1, var2]);

        let var3 = new_variable(&mut a);

        let my_env = Env(hashmap![
            "pair".to_string() => {
                new_function(&mut a, &[var1, var2], pair_type)
            },
            "true".to_string() => 1,
            "cond".to_string() => {
                new_function(&mut a, &[1, var3, var3], var3)
            },
            "zero".to_string() => new_function(&mut a, &[0], 1),
            "pred".to_string() => new_function(&mut a, &[0], 0),
            "times".to_string() => {
                new_function(&mut a, &[0, 0], 0)
            },
        ]);

        (a, my_env)
    }

    /// Sets up some predefined types using the type constructors TypeVariable,
    /// TypeOperator and Function.  Creates a list of example expressions to be
    /// evaluated. Evaluates the expressions, printing the type or errors arising
    /// from each.

    #[test]
    fn test_factorial() {
        let (mut a, my_env) = test_env();

        // factorial
        let syntax = new_letrec(
            "factorial", // letrec factorial =
            new_lambda(
                &["n"], // fn n =>
                new_apply(
                    new_identifier("cond"), // cond(zero(n), 1, times(n, factorial(pred(n)))
                    &[
                        new_apply(new_identifier("zero"), &[new_identifier("n")]),
                        new_identifier("1"),
                        new_apply(
                            // times(n, factorial(pred(n))
                            new_identifier("times"),
                            &[
                                new_identifier("n"),
                                new_apply(
                                    new_identifier("factorial"),
                                    &[new_apply(new_identifier("pred"), &[new_identifier("n")])],
                                ),
                            ],
                        ),
                    ],
                ),
            ), // in
            new_apply(new_identifier("factorial"), &[new_identifier("5")]),
        );

        let t = analyse(&mut a, &syntax, &my_env, &hashset![]);
        assert_eq!(
            a[t].as_string(
                &a,
                &mut Namer {
                    value: 'a',
                    set: hashmap![],
                }
            ),
            r#"int"#
        );
    }

    #[should_panic]
    #[test]
    fn test_mismatch() {
        let (mut a, my_env) = test_env();

        // fn x => (pair(x(3) (x(true)))
        let syntax = new_lambda(
            &["x"],
            new_apply(
                new_identifier("pair"),
                &[
                    new_apply(new_identifier("x"), &[new_identifier("3")]),
                    new_apply(new_identifier("x"), &[new_identifier("true")]),
                ],
            ),
        );

        let _ = analyse(&mut a, &syntax, &my_env, &hashset![]);
    }

    #[should_panic = "Undefined symbol \"f\""]
    #[test]
    fn test_pair() {
        let (mut a, my_env) = test_env();

        // pair(f(3), f(true))
        let syntax = new_apply(
            new_identifier("pair"),
            &[
                new_apply(new_identifier("f"), &[new_identifier("4")]),
                new_apply(new_identifier("f"), &[new_identifier("true")]),
            ],
        );

        let _ = analyse(&mut a, &syntax, &my_env, &hashset![]);
    }

    #[test]
    fn test_mul() {
        let (mut a, my_env) = test_env();

        let pair = new_apply(
            new_identifier("pair"),
            &[
                new_apply(new_identifier("f"), &[new_identifier("4")]),
                new_apply(new_identifier("f"), &[new_identifier("true")]),
            ],
        );

        // let f = (fn x => x) in ((pair (f 4)) (f true))
        let syntax = new_let("f", new_lambda(&["x"], new_identifier("x")), pair);

        let t = analyse(&mut a, &syntax, &my_env, &hashset![]);
        assert_eq!(
            a[t].as_string(
                &a,
                &mut Namer {
                    value: 'a',
                    set: hashmap![],
                }
            ),
            r#"(int * bool)"#
        );
    }

    #[should_panic = "recursive unification"]
    #[test]
    fn test_recursive() {
        let (mut a, my_env) = test_env();

        // fn f => f f (fail)
        let syntax = new_lambda(
            &["f"],
            new_apply(new_identifier("f"), &[new_identifier("f")]),
        );

        let t = analyse(&mut a, &syntax, &my_env, &hashset![]);
        assert_eq!(
            a[t].as_string(
                &a,
                &mut Namer {
                    value: 'a',
                    set: hashmap![],
                }
            ),
            r#"int"#
        );
    }

    #[test]
    fn test_int() {
        let (mut a, my_env) = test_env();

        // let g = fn f => 5 in g g
        let syntax = new_let(
            "g",
            new_lambda(&["f"], new_identifier("5")),
            new_apply(new_identifier("g"), &[new_identifier("g")]),
        );

        let t = analyse(&mut a, &syntax, &my_env, &hashset![]);
        assert_eq!(
            a[t].as_string(
                &a,
                &mut Namer {
                    value: 'a',
                    set: hashmap![],
                }
            ),
            r#"int"#
        );
    }

    #[test]
    fn test_generic_nongeneric() {
        let (mut a, my_env) = test_env();

        // example that demonstrates generic and non-generic variables:
        // fn g => let f = fn x => g in pair (f 3, f true)
        let syntax = new_lambda(
            &["g"],
            new_let(
                "f",
                new_lambda(&["x"], new_identifier("g")),
                new_apply(
                    new_identifier("pair"),
                    &[
                        new_apply(new_identifier("f"), &[new_identifier("3")]),
                        new_apply(new_identifier("f"), &[new_identifier("true")]),
                    ],
                ),
            ),
        );

        let t = analyse(&mut a, &syntax, &my_env, &hashset![]);
        assert_eq!(
            a[t].as_string(
                &a,
                &mut Namer {
                    value: 'a',
                    set: hashmap![],
                }
            ),
            r#"(a -> (a * a))"#
        );
    }

    #[test]
    fn test_composition() {
        let (mut a, my_env) = test_env();

        // Function composition
        // fn f (fn g (fn arg (f g arg)))
        let syntax = new_lambda(
            &["f"],
            new_lambda(
                &["g"],
                new_lambda(
                    &["arg"],
                    new_apply(
                        new_identifier("g"),
                        &[new_apply(new_identifier("f"), &[new_identifier("arg")])],
                    ),
                ),
            ),
        );

        let t = analyse(&mut a, &syntax, &my_env, &hashset![]);
        assert_eq!(
            a[t].as_string(
                &a,
                &mut Namer {
                    value: 'a',
                    set: hashmap![],
                }
            ),
            r#"((a -> b) -> ((b -> c) -> (a -> c)))"#
        );
    }

    #[test]
    fn test_fun() {
        let (mut a, my_env) = test_env();

        // Function composition
        // (fn (f, g, arg) -> (f g arg))
        let syntax = new_lambda(
            &["f", "g", "arg"],
            new_apply(
                new_identifier("g"),
                &[new_apply(new_identifier("f"), &[new_identifier("arg")])],
            ),
        );

        let t = analyse(&mut a, &syntax, &my_env, &hashset![]);
        assert_eq!(
            a[t].as_string(
                &a,
                &mut Namer {
                    value: 'a',
                    set: hashmap![],
                }
            ),
            r#"((a -> b), (b -> c), a -> c)"#
        );
    }
}