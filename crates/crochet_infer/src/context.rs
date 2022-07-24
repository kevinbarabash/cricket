use std::cell::Cell;
use std::collections::HashMap;

use crochet_ast::literal::Lit as AstLit;

use super::substitutable::*;
use super::types::*;

// This maps to the Assump data type in THIH which was a tuple
// of (Id, Scheme) where Id was a String.
pub type Env = HashMap<String, Scheme>;

#[derive(Clone, Debug)]
pub struct State {
    pub count: Cell<i32>,
}

#[derive(Clone, Debug)]
pub struct Context {
    pub values: Env,
    pub types: Env,
    pub state: State,
    pub is_async: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            types: HashMap::new(),
            state: State {
                count: Cell::from(0),
            },
            is_async: false,
        }
    }
}

impl Context {
    pub fn lookup_value(&self, name: &str) -> Result<Type, String> {
        let scheme = self
            .values
            .get(name)
            .ok_or(format!("Can't find type: {name}"))?;
        Ok(self.instantiate(scheme))
    }
    // TODO: Make this return a Result<Type, String>
    pub fn lookup_type(&self, name: &str) -> Result<Type, String> {
        let scheme = self
            .types
            .get(name)
            .ok_or(format!("Can't find type: {name}"))?;
        Ok(self.instantiate(scheme))
    }

    pub fn instantiate(&self, scheme: &Scheme) -> Type {
        let ids = scheme.qualifiers.iter().map(|id| id.to_owned());
        let fresh_quals = scheme.qualifiers.iter().map(|_| self.fresh_var());
        let subs: Subst = ids.zip(fresh_quals).collect();

        scheme.ty.apply(&subs)
    }

    pub fn fresh_id(&self) -> i32 {
        let id = self.state.count.get() + 1;
        self.state.count.set(id);
        id
    }

    pub fn fresh_var(&self) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Var,
            flag: None,
        }
    }

    pub fn lam(&self, params: Vec<Type>, ret: Box<Type>) -> Type {
        self.lam_with_option_flag(params, ret, None)
    }
    pub fn lam_with_flag(&self, params: Vec<Type>, ret: Box<Type>, flag: Flag) -> Type {
        self.lam_with_option_flag(params, ret, Some(flag))
    }
    fn lam_with_option_flag(&self, params: Vec<Type>, ret: Box<Type>, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Lam(LamType {
                params,
                ret,
                is_call: false,
            }),
            flag,
        }
    }

    pub fn prim(&self, prim: Primitive) -> Type {
        self.prim_with_option_flag(prim, None)
    }
    pub fn prim_with_flag(&self, prim: Primitive, flag: Flag) -> Type {
        self.prim_with_option_flag(prim, Some(flag))
    }
    fn prim_with_option_flag(&self, prim: Primitive, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Prim(prim),
            flag,
        }
    }

    pub fn lit(&self, lit: AstLit) -> Type {
        self.lit_with_option_flag(lit, None)
    }
    pub fn lit_with_flag(&self, lit: AstLit, flag: Flag) -> Type {
        self.lit_with_option_flag(lit, Some(flag))
    }
    fn lit_with_option_flag(&self, lit: AstLit, flag: Option<Flag>) -> Type {
        let lit = match lit {
            AstLit::Num(n) => Lit::Num(n.value),
            AstLit::Bool(b) => Lit::Bool(b.value),
            AstLit::Str(s) => Lit::Str(s.value),
            AstLit::Null(_) => Lit::Null,
            AstLit::Undefined(_) => Lit::Undefined,
        };
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Lit(lit),
            flag,
        }
    }

    pub fn lit_type(&self, lit: Lit) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Lit(lit),
            flag: None,
        }
    }

    pub fn union(&self, types: Vec<Type>) -> Type {
        self.union_with_option_flag(types, None)
    }
    pub fn union_with_flag(&self, types: Vec<Type>, flag: Flag) -> Type {
        self.union_with_option_flag(types, Some(flag))
    }
    fn union_with_option_flag(&self, types: Vec<Type>, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Union(types),
            flag,
        }
    }

    pub fn intersection(&self, types: Vec<Type>) -> Type {
        self.intersection_with_option_flag(types, None)
    }
    pub fn intersection_with_flag(&self, types: Vec<Type>, flag: Flag) -> Type {
        self.intersection_with_option_flag(types, Some(flag))
    }
    fn intersection_with_option_flag(&self, types: Vec<Type>, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Intersection(types),
            flag,
        }
    }

    pub fn object(&self, props: Vec<TProp>) -> Type {
        self.object_with_option_flag(props, None)
    }
    pub fn object_with_flag(&self, props: Vec<TProp>, flag: Flag) -> Type {
        self.object_with_option_flag(props, Some(flag))
    }
    fn object_with_option_flag(&self, props: Vec<TProp>, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Object(props),
            flag,
        }
    }

    pub fn prop(&self, name: &str, ty: Type, optional: bool) -> TProp {
        TProp {
            name: name.to_owned(),
            optional,
            ty,
        }
    }

    pub fn alias(&self, name: &str, type_params: Option<Vec<Type>>) -> Type {
        self.alias_with_option_flag(name, type_params, None)
    }
    pub fn alias_with_flag(&self, name: &str, type_params: Option<Vec<Type>>, flag: Flag) -> Type {
        self.alias_with_option_flag(name, type_params, Some(flag))
    }
    fn alias_with_option_flag(
        &self,
        name: &str,
        type_params: Option<Vec<Type>>,
        flag: Option<Flag>,
    ) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Alias(AliasType {
                name: name.to_owned(),
                type_params,
            }),
            flag,
        }
    }

    pub fn tuple(&self, types: Vec<Type>) -> Type {
        self.tuple_with_option_flag(types, None)
    }
    pub fn tuple_with_flag(&self, types: Vec<Type>, flag: Flag) -> Type {
        self.tuple_with_option_flag(types, Some(flag))
    }
    fn tuple_with_option_flag(&self, types: Vec<Type>, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Tuple(types),
            flag,
        }
    }

    pub fn array(&self, t: Type) -> Type {
        self.array_with_option_flag(t, None)
    }
    pub fn array_with_flag(&self, t: Type, flag: Flag) -> Type {
        self.array_with_option_flag(t, Some(flag))
    }
    fn array_with_option_flag(&self, t: Type, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Array(Box::from(t)),
            flag,
        }
    }

    pub fn rest(&self, arg: Type) -> Type {
        self.rest_with_option_flag(arg, None)
    }
    pub fn rest_with_flag(&self, arg: Type, flag: Flag) -> Type {
        self.rest_with_option_flag(arg, Some(flag))
    }
    fn rest_with_option_flag(&self, arg: Type, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Rest(Box::from(arg)),
            flag,
        }
    }

    pub fn mem(&self, obj: Type, prop: &str) -> Type {
        self.mem_with_option_flag(obj, prop, None)
    }
    pub fn mem_with_flag(&self, obj: Type, prop: &str, flag: Flag) -> Type {
        self.mem_with_option_flag(obj, prop, Some(flag))
    }
    fn mem_with_option_flag(&self, obj: Type, prop: &str, flag: Option<Flag>) -> Type {
        Type {
            id: self.fresh_id(),
            frozen: false,
            variant: Variant::Member(MemberType {
                obj: Box::from(obj),
                prop: prop.to_owned(),
            }),
            flag,
        }
    }
}

pub fn lookup_alias(ctx: &Context, alias: &AliasType) -> Result<Type, String> {
    match ctx.types.get(&alias.name) {
        Some(scheme) => {
            // Replaces qualifiers in the scheme with the corresponding type params
            // from the alias type.
            let ids = scheme.qualifiers.iter().map(|id| id.to_owned());
            let subs: Subst = match &alias.type_params {
                Some(type_params) => {
                    if scheme.qualifiers.len() != type_params.len() {
                        return Err(String::from("mismatch between number of qualifiers in scheme and number of type params"));
                    }
                    ids.zip(type_params.iter().cloned()).collect()
                },
                None => {
                    if !scheme.qualifiers.is_empty() {
                        return Err(String::from("mismatch between number of qualifiers in scheme and number of type params"));
                    }
                    ids.zip(scheme.qualifiers.iter().map(|_| ctx.fresh_var())).collect()
                },
            };

            Ok(scheme.ty.apply(&subs))
        },
        None => Err(String::from("Can't find alias '{name}' in context")),
    }
}
