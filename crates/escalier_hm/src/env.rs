use std::collections::{HashMap, HashSet};

use crate::errors::*;
use crate::types::*;
use crate::util::*;

#[derive(Clone, Debug, Default)]
pub struct Env(pub HashMap<String, ArenaType>);

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
pub fn get_type(
    a: &mut Vec<Type>,
    name: &str,
    env: &Env,
    non_generic: &HashSet<ArenaType>,
) -> Result<ArenaType, Errors> {
    if let Some(value) = env.0.get(name) {
        let mat = non_generic.iter().cloned().collect::<Vec<_>>();
        Ok(fresh(a, *value, &mat))
    } else {
        Err(Errors::InferenceError(format!(
            "Undefined symbol {:?}",
            name
        )))
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
pub fn fresh(a: &mut Vec<Type>, t: ArenaType, non_generic: &[ArenaType]) -> ArenaType {
    // A mapping of TypeVariables to TypeVariables
    let mut mappings = HashMap::default();

    fn freshrec(
        a: &mut Vec<Type>,
        tp: ArenaType,
        mappings: &mut HashMap<String, ArenaType>,
        non_generic: &[ArenaType],
    ) -> ArenaType {
        let p = prune(a, tp);
        // We clone here because we can't move out of a shared reference.
        // TODO: Consider using Rc<RefCell<Type>> to avoid unnecessary cloning.
        match &a.get(p).unwrap().clone().kind {
            TypeKind::Variable(_) => {
                // if is_generic(a, p, non_generic) {
                //     mappings
                //         .entry(p)
                //         .or_insert_with(|| new_var_type(a))
                //         .to_owned()
                // } else {
                //     p
                // }
                p
            }
            TypeKind::Constructor(con) => {
                let types = freshrec_many(a, &con.types, mappings, non_generic);
                new_constructor(a, &con.name, &types)
            }
            TypeKind::Ref(tref) => match mappings.get(&tref.name) {
                Some(t) => t.to_owned(),
                None => new_type_ref(a, &tref.name),
            },
            TypeKind::Literal(lit) => new_lit_type(a, lit),
            TypeKind::Tuple(tuple) => {
                let types = freshrec_many(a, &tuple.types, mappings, non_generic);
                new_tuple_type(a, &types)
            }
            TypeKind::Object(object) => {
                let fields: Vec<_> = object
                    .props
                    .iter()
                    .map(|(name, tp)| (name.clone(), freshrec(a, *tp, mappings, non_generic)))
                    .collect();
                new_object_type(a, &fields)
            }
            TypeKind::Function(func) => {
                // TODO: instantiate the function if it's generic
                eprintln!("func.type_params = {:?}", func.type_params);

                if let Some(type_params) = &func.type_params {
                    for tp in type_params {
                        mappings
                            .entry(tp.name.to_owned())
                            .or_insert_with(|| new_var_type(a));
                    }
                }

                let params = freshrec_many(a, &func.params, mappings, non_generic);
                let ret = freshrec(a, func.ret, mappings, non_generic);

                new_func_type(a, &params, ret, None)
            }
            TypeKind::Union(union) => {
                let types = freshrec_many(a, &union.types, mappings, non_generic);
                new_union_type(a, &types)
            }
        }
    }

    pub fn freshrec_many(
        a: &mut Vec<Type>,
        types: &[ArenaType],
        mappings: &mut HashMap<String, ArenaType>,
        non_generic: &[ArenaType],
    ) -> Vec<ArenaType> {
        types
            .iter()
            .map(|x| freshrec(a, *x, mappings, non_generic))
            .collect()
    }

    freshrec(a, t, &mut mappings, non_generic)
}
