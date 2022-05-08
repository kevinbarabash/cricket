use super::constraint_solver::{run_solve, Constraint};
use super::substitutable::*;
use super::types::*;

use std::collections::{HashMap, HashSet};
use std::iter::Iterator;

// TODO: adopt immutable collections: https://github.com/bodil/im-rs
// this will save on copying data when cloning of collections is required

impl Substitutable for Constraint {
    fn apply(&self, sub: &Subst) -> Constraint {
        Constraint {
            types: (self.types.0.apply(sub), self.types.1.apply(sub)),
        }
    }
    fn ftv(&self) -> HashSet<i32> {
        let mut result = HashSet::new();
        result.extend(self.types.0.ftv());
        result.extend(self.types.1.ftv());
        result
    }
}

impl Substitutable for Type {
    fn apply(&self, sub: &Subst) -> Type {
        match self {
            Type {
                id,
                kind: TypeKind::Var,
                ..
            } => sub.get(id).unwrap_or(self).clone(),
            // TODO: handle widening of lambdas
            Type {
                id,
                frozen,
                kind: TypeKind::Lam(TLam { args, ret }),
            } => Type {
                id: id.to_owned(),
                frozen: frozen.to_owned(),
                kind: TypeKind::Lam(TLam {
                    args: args.iter().map(|arg| arg.apply(sub)).collect(),
                    ret: Box::from(ret.apply(sub)),
                }),
            },
            Type {
                id,
                kind: TypeKind::Prim(_),
                ..
            } => sub.get(id).unwrap_or(self).clone(),
            Type {
                id,
                kind: TypeKind::Lit(_),
                ..
            } => sub.get(id).unwrap_or(self).clone(),
            // TODO: handle widening of unions
            Type {
                id,
                frozen,
                kind: TypeKind::Union(types),
            } => Type {
                id: id.to_owned(),
                frozen: frozen.to_owned(),
                kind: TypeKind::Union(types.iter().map(|ty| ty.apply(sub)).collect())
            }
        }
    }
    fn ftv(&self) -> HashSet<i32> {
        match self {
            Type {
                id,
                kind: TypeKind::Var,
                ..
            } => HashSet::from([id.to_owned()]),
            Type {
                kind: TypeKind::Lam(TLam { args, ret }),
                ..
            } => {
                let mut result: HashSet<_> = args.iter().flat_map(|a| a.ftv()).collect();
                result.extend(ret.ftv());
                result
            }
            Type {
                kind: TypeKind::Prim(_),
                ..
            } => HashSet::new(),
            Type {
                kind: TypeKind::Lit(_),
                ..
            } => HashSet::new(),
            Type {
                kind: TypeKind::Union(types),
                ..
            } => types.iter().flat_map(|ty| ty.ftv()).collect()
        }
    }
}

impl Substitutable for Scheme {
    fn apply(&self, sub: &Subst) -> Scheme {
        Scheme {
            qualifiers: self.qualifiers.clone(),
            ty: self.ty.apply(sub),
        }
    }
    fn ftv(&self) -> HashSet<i32> {
        let qualifiers: HashSet<_> = self.qualifiers.iter().map(|id| id.to_owned()).collect();
        self.ty.ftv().difference(&qualifiers).cloned().collect()
    }
}

impl Substitutable for Env {
    fn apply(&self, sub: &Subst) -> Env {
        self.iter()
            .map(|(a, b)| (a.clone(), b.apply(sub)))
            .collect()
    }
    fn ftv(&self) -> HashSet<i32> {
        // we can't use iter_values() here because it's a consuming iterator
        self.iter().flat_map(|(_, b)| b.ftv()).collect()
    }
}

impl<I> Substitutable for Vec<I>
where
    I: Substitutable,
{
    fn apply(&self, sub: &Subst) -> Vec<I> {
        self.iter().map(|c| c.apply(sub)).collect()
    }
    fn ftv(&self) -> HashSet<i32> {
        self.iter().flat_map(|c| c.ftv()).collect()
    }
}

use crate::context::{Context, Env};
use crate::syntax::{BindingIdent, Expr, Pattern, Program, Statement, WithSpan};

// TODO: We need multiple Envs so that we can control things at differen scopes
// e.g. global, module, function, ...
pub fn infer_prog(env: Env, prog: &Program) -> Env {
    let mut ctx: Context = Context::from(env);

    for (stmt, _span) in &prog.body {
        match stmt {
            Statement::Decl {
                pattern: (Pattern::Ident { name }, _span),
                value,
            } => {
                let scheme = infer_expr(&ctx, value);
                ctx.env.insert(name.to_owned(), scheme);
            }
            Statement::Expr(expr) => {
                // We ignore the type that was inferred, we only care that
                // it succeeds since we aren't assigning it to variable.
                infer_expr(&ctx, expr);
            }
        };
    }

    ctx.env
}

pub fn infer_stmt(ctx: &Context, stmt: &WithSpan<Statement>) -> Scheme {
    match stmt {
        (Statement::Expr(e), _) => infer_expr(ctx, e),
        _ => panic!("We can't infer decls yet"),
    }
}

pub fn infer_expr(ctx: &Context, expr: &WithSpan<Expr>) -> Scheme {
    let (ty, cs) = infer(expr, ctx);
    let subs = run_solve(&cs, ctx);

    close_over(&ty.apply(&subs))
}

fn close_over(ty: &Type) -> Scheme {
    let empty_env = Env::new();
    normalize(&generalize(&empty_env, ty))
}

fn normalize(sc: &Scheme) -> Scheme {
    let body = &sc.ty;
    let keys = body.ftv();
    let mut keys: Vec<_> = keys.iter().cloned().collect();
    keys.sort();
    let mapping: HashMap<i32, Type> = keys
        .iter()
        .enumerate()
        .map(|(index, key)| {
            (
                key.to_owned(),
                Type {
                    id: index as i32,
                    kind: TypeKind::Var,
                    frozen: false,
                },
            )
        })
        .collect();

    fn norm_type(ty: &Type, mapping: &HashMap<i32, Type>) -> Type {
        match ty {
            Type {
                id,
                kind: TypeKind::Var,
                ..
            } => mapping.get(&id).unwrap().to_owned(),
            Type {
                id,
                frozen,
                kind: TypeKind::Lam(TLam { args, ret }),
            } => {
                let args: Vec<_> = args.iter().map(|arg| norm_type(arg, mapping)).collect();
                let ret = Box::from(norm_type(ret, mapping));
                Type {
                    id: id.to_owned(),
                    frozen: frozen.to_owned(),
                    kind: TypeKind::Lam(TLam { args, ret }),
                }
            }
            Type {
                kind: TypeKind::Prim(_),
                ..
            } => ty.to_owned(),
            Type {
                kind: TypeKind::Lit(_),
                ..
            } => ty.to_owned(),
            Type {
                id,
                frozen,
                kind: TypeKind::Union(types),
            } => {
                let types = types.iter().map(|ty| norm_type(ty, mapping)).collect();
                Type {
                    id: id.to_owned(),
                    frozen: frozen.to_owned(),
                    kind: TypeKind::Union(types),
                }
            }
        }
    }

    Scheme {
        qualifiers: (0..keys.len()).map(|x| x as i32).collect(),
        ty: norm_type(body, &mapping),
    }
}

fn generalize(env: &Env, ty: &Type) -> Scheme {
    // ftv() returns a Set which is not ordered
    // TODO: switch to an ordered set
    let mut qualifiers: Vec<_> = ty.ftv().difference(&env.ftv()).cloned().collect();
    qualifiers.sort();
    Scheme {
        qualifiers,
        ty: ty.clone(),
    }
}

type InferResult = (Type, Vec<Constraint>);

fn infer(expr: &WithSpan<Expr>, ctx: &Context) -> InferResult {
    match expr {
        (Expr::Ident { name }, _) => {
            let ty = ctx.lookup_env(name);
            (ty, vec![])
        }
        (Expr::App { lam, args }, _) => {
            let (t_fn, cs_fn) = infer(lam, ctx);
            let (t_args, cs_args) = infer_many(args, ctx);
            let tv = ctx.fresh_tvar();

            let mut constraints = Vec::new();
            constraints.extend(cs_fn);
            constraints.extend(cs_args);
            constraints.push(Constraint {
                types: (
                    ctx.from_lam(TLam {
                        args: t_args,
                        ret: Box::new(tv.clone()),
                    }),
                    t_fn,
                ),
            });

            (tv, constraints)
        }
        (Expr::Fix { expr }, _) => {
            let (t, cs) = infer(expr, ctx);
            let tv = ctx.fresh_tvar();
            let mut constraints = Vec::new();
            constraints.extend(cs);
            constraints.push(Constraint {
                types: (
                    ctx.from_lam(TLam {
                        args: vec![tv.clone()],
                        ret: Box::new(tv.clone()),
                    }),
                    t,
                ),
            });

            (tv, constraints)
        }
        (
            Expr::If {
                cond,
                consequent,
                alternate,
            },
            _,
        ) => {
            let (t1, cs1) = infer(cond, ctx);
            let (t2, cs2) = infer(consequent, ctx);
            let (t3, cs3) = infer(alternate, ctx);
            let bool = ctx.from_prim(Primitive::Bool);

            let result_type = t2.clone();
            let mut constraints = Vec::new();
            constraints.extend(cs1);
            constraints.extend(cs2);
            constraints.extend(cs3);
            constraints.push(Constraint { types: (t1, bool) });
            constraints.push(Constraint { types: (t2, t3) });

            (result_type, constraints)
        }
        (Expr::Lam { args, body, .. }, _) => {
            // Creates a new type variable for each arg
            let arg_tvs: Vec<_> = args.iter().map(|_| ctx.fresh_tvar()).collect();
            let mut new_ctx = ctx.clone();
            for (arg, tv) in args.iter().zip(arg_tvs.clone().into_iter()) {
                let scheme = Scheme {
                    qualifiers: vec![],
                    ty: tv.clone(),
                };
                match arg {
                    (BindingIdent::Ident { name }, _) => {
                        new_ctx.env.insert(name.to_string(), scheme)
                    }
                    (BindingIdent::Rest { name }, _) => {
                        new_ctx.env.insert(name.to_string(), scheme)
                    }
                };
            }
            let (ret_ty, cs) = infer(body, &new_ctx);
            ctx.state.count.set(new_ctx.state.count.get());
            let lam_ty = ctx.from_lam(TLam {
                args: arg_tvs,
                ret: Box::new(ret_ty),
            });

            (lam_ty, cs)
        }
        (
            Expr::Let {
                pattern,
                value,
                body,
            },
            _,
        ) => {
            let (t1, cs1) = infer(&value, &ctx);
            let subs = run_solve(&cs1, &ctx);
            let (new_ctx, new_cs) = infer_pattern(pattern, &t1, &subs, ctx);
            let (t2, cs2) = infer(body, &new_ctx);
            ctx.state.count.set(new_ctx.state.count.get());
            let mut cs: Vec<Constraint> = Vec::new();
            cs.extend(cs1);
            cs.extend(new_cs);
            cs.extend(cs2.apply(&subs));

            (t2.apply(&subs), vec![])
        }
        (Expr::Lit { literal }, _) => (ctx.from_lit(literal.to_owned()), vec![]),
        // TODO: check the `op` field when we introduce comparison operators
        (Expr::Op { left, right, .. }, _) => {
            let left = Box::as_ref(left);
            let right = Box::as_ref(right);
            let (ts, cs) = infer_many(&[left.clone(), right.clone()], ctx);
            let tv = ctx.fresh_tvar();

            let mut cs = cs;
            let c = Constraint {
                types: (
                    ctx.from_lam(TLam {
                        args: ts,
                        ret: Box::from(tv.clone()),
                    }),
                    ctx.from_lam(TLam {
                        args: vec![ctx.from_prim(Primitive::Num), ctx.from_prim(Primitive::Num)],
                        ret: Box::from(ctx.from_prim(Primitive::Num)),
                    }),
                ),
            };
            cs.push(c);

            (tv, cs)
        }
    }
}

fn infer_pattern(
    pattern: &WithSpan<Pattern>,
    ty: &Type,
    subs: &Subst,
    ctx: &Context,
) -> (Context, Vec<Constraint>) {
    let scheme = generalize(&ctx.env.apply(subs), &ty.apply(subs));

    match pattern {
        (Pattern::Ident { name }, _) => {
            let mut new_ctx = ctx.clone();
            new_ctx.env.insert(name.to_owned(), scheme);

            (new_ctx, vec![])
        }
    }
}

fn infer_many(exprs: &[WithSpan<Expr>], ctx: &Context) -> (Vec<Type>, Vec<Constraint>) {
    let mut ts: Vec<Type> = Vec::new();
    let mut all_cs: Vec<Constraint> = Vec::new();

    for elem in exprs {
        let (ty, cs) = infer(elem, ctx);
        ts.push(ty);
        all_cs.extend(cs);
    }

    (ts, all_cs)
}
