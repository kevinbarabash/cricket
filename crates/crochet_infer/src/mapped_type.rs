use crochet_ast::types::{
    BindingIdent, TFnParam, TIndex, TIndexAccess, TMappedType, TObjElem, TObject, TPat, TProp,
    Type, TypeKind,
};
use error_stack::{Report, Result};

use crate::context::Context;
use crate::key_of::key_of;
use crate::type_error::TypeError;
use crate::visitor::Visitor;

// unwraps `t` recursively until it finds an object type or something that is
// definitely not an object type
// TODO: update `infer_property_type` to use this helper function
fn unwrap_obj_type(t: &Type, ctx: &Context) -> Result<Type, TypeError> {
    let t = match &t.kind {
        TypeKind::Var(_) => todo!(),
        TypeKind::App(_) => todo!(),
        TypeKind::Lam(_) => todo!(),
        TypeKind::Lit(_) => todo!(), // TODO: lookup Number, String, Boolean in Context
        TypeKind::Keyword(_) => todo!(), // TODO: lookup Number, String, Boolean in Context
        TypeKind::Union(_) => todo!(),
        TypeKind::Intersection(_) => todo!(),
        TypeKind::Object(_) => t.to_owned(),
        TypeKind::Ref(alias) => unwrap_obj_type(&ctx.lookup_ref_and_instantiate(alias)?, ctx)?,
        TypeKind::Tuple(_) => todo!(), // TODO: lookup Array in Context
        TypeKind::Array(_) => todo!(), // TODO: lookup Array in Context
        TypeKind::Rest(_) => todo!(),
        TypeKind::This => todo!(),
        TypeKind::KeyOf(_) => todo!(),
        TypeKind::IndexAccess(_) => todo!(),
        TypeKind::MappedType(_) => todo!(),
        TypeKind::Generic(_) => todo!(),
    };
    Ok(t)
}

fn get_obj_type_from_mapped_type(mapped: &TMappedType, ctx: &Context) -> Result<Type, TypeError> {
    // TODO:
    // - check if mapped.type_param.constraint is a keyof type
    // - if not, look at the constraint's provenence to see it was a keyof type
    // - as a last resort, look at mapped.t to see if it's an indexed access type
    if let Some(constraint) = &mapped.type_param.constraint {
        if let TypeKind::KeyOf(t) = &constraint.kind {
            return unwrap_obj_type(t.as_ref(), ctx);
        }

        // TODO: look at constraint.provenence to see if it's a keyof type
    } else if let TypeKind::IndexAccess(access) = &mapped.t.kind {
        return unwrap_obj_type(access.object.as_ref(), ctx);
    }

    Err(Report::new(TypeError::Unhandled))
}

fn get_prop(obj: &TObject, key: &str) -> Result<TProp, TypeError> {
    for elem in &obj.elems {
        if let TObjElem::Prop(prop) = elem {
            if prop.name == key {
                return Ok(prop.to_owned());
            }
        }
    }

    Err(Report::new(TypeError::MissingKey(key.to_owned())))
}

fn computed_indexed_access(access: &TIndexAccess, ctx: &Context) -> Result<Type, TypeError> {
    let obj = unwrap_obj_type(access.object.as_ref(), ctx)?;
    let index = access.index.as_ref();

    let key = match &index.kind {
        TypeKind::Lit(lit) => match lit {
            crochet_ast::types::TLit::Num(num) => num,
            crochet_ast::types::TLit::Bool(_) => todo!(),
            crochet_ast::types::TLit::Str(str) => str,
        },
        _ => {
            return Err(Report::new(TypeError::InvalidIndex(
                access.object.to_owned(),
                access.index.to_owned(),
            )));
        }
    };

    if let TypeKind::Object(obj) = &obj.kind {
        let prop = get_prop(obj, key)?;
        return Ok(prop.t);
    }

    Err(Report::new(TypeError::Unhandled))
}

fn get_prop_by_name(elems: &[TObjElem], name: &str) -> Result<TProp, TypeError> {
    for elem in elems {
        match elem {
            TObjElem::Call(_) => (),
            TObjElem::Constructor(_) => (),
            TObjElem::Index(_) => (),
            TObjElem::Prop(prop) => {
                if prop.name == name {
                    return Ok(prop.to_owned());
                }
            }
        }
    }

    Err(Report::new(TypeError::MissingKey(name.to_owned())))
}

pub fn compute_mapped_type(t: &Type, ctx: &Context) -> Result<Type, TypeError> {
    match &t.kind {
        TypeKind::Ref(alias) => {
            let t = ctx.lookup_ref_and_instantiate(alias)?;
            compute_mapped_type(&t, ctx)
        }
        TypeKind::MappedType(mapped) => {
            let constraint = mapped.type_param.constraint.as_ref().unwrap();
            let keys = key_of(constraint, ctx)?;
            let obj = get_obj_type_from_mapped_type(mapped, ctx)?;

            let elems = match &obj.kind {
                TypeKind::Object(TObject { elems }) => elems.to_owned(),
                _ => vec![],
            };

            // keys is either a union of all the prop keys/indexer types or
            // is a single prop key/indexer type.
            if let TypeKind::Union(keys) = keys.kind {
                let elems = keys
                    .iter()
                    .map(|key| {
                        let mut value = mapped.t.clone();

                        // if key is a:
                        // - number, string, or symbol then create an indexer
                        // - literal of those types then create a normal property
                        replace_tvar(&mut value, &mapped.type_param.id, key);

                        let value = match &value.kind {
                            TypeKind::IndexAccess(access) => computed_indexed_access(access, ctx)?,
                            _ => value.as_ref().to_owned(),
                        };

                        match &key.kind {
                            TypeKind::Lit(lit) => match lit {
                                crochet_ast::types::TLit::Num(name) => {
                                    let prop = get_prop_by_name(&elems, name)?;
                                    let optional = match &mapped.optional {
                                        Some(change) => match change {
                                            crochet_ast::types::TMappedTypeChangeProp::Plus => true,
                                            crochet_ast::types::TMappedTypeChangeProp::Minus => false,
                                        },
                                        None => prop.optional,
                                    };
                                    let mutable = match &mapped.mutable {
                                        Some(change) => match change {
                                            crochet_ast::types::TMappedTypeChangeProp::Plus => true,
                                            crochet_ast::types::TMappedTypeChangeProp::Minus => false,
                                        },
                                        None => prop.mutable,
                                    };
                                    Ok(TObjElem::Prop(TProp {
                                        name: name.to_owned(),
                                        optional,
                                        mutable,
                                        t: value,
                                    }))
                                }
                                crochet_ast::types::TLit::Bool(_) => {
                                    Err(Report::new(TypeError::Unhandled))
                                }
                                crochet_ast::types::TLit::Str(name) => {
                                    let prop = get_prop_by_name(&elems, name)?;
                                    let optional = match &mapped.optional {
                                        Some(change) => match change {
                                            crochet_ast::types::TMappedTypeChangeProp::Plus => true,
                                            crochet_ast::types::TMappedTypeChangeProp::Minus => false,
                                        },
                                        None => prop.optional,
                                    };
                                    let mutable = match &mapped.mutable {
                                        Some(change) => match change {
                                            crochet_ast::types::TMappedTypeChangeProp::Plus => true,
                                            crochet_ast::types::TMappedTypeChangeProp::Minus => false,
                                        },
                                        None => prop.mutable,
                                    };
                                    Ok(TObjElem::Prop(TProp {
                                        name: name.to_owned(),
                                        optional,
                                        mutable,
                                        t: value,
                                    }))
                                }
                            },
                            // TODO: get indexer(s), you can mix symbol + number
                            // OR symbol + string, but not number + string.
                            TypeKind::Keyword(_) => Ok(TObjElem::Index(TIndex {
                                key: TFnParam {
                                    pat: TPat::Ident(BindingIdent {
                                        name: String::from("key"),
                                        mutable: false, // TODO
                                    }),
                                    t: key.to_owned(),
                                    optional: false, // TODO
                                },
                                // How do we maintain the optionality of each property
                                // when we aren't setting it explicitly
                                mutable: false, // TODO
                                t: value,
                            })),
                            _ => Err(Report::new(TypeError::Unhandled)),
                        }
                    })
                    .collect::<Result<Vec<_>, TypeError>>()?;

                let t = Type {
                    kind: TypeKind::Object(TObject { elems }),
                    mutable: false,
                    provenance: None, // TODO: fill this in
                };

                Ok(t)
            } else {
                // TODO: handle there being only a single key, create a helper
                // function for getting the keys as a vector.
                Err(Report::new(TypeError::Unhandled))
            }
        }
        _ => Err(Report::new(TypeError::Unhandled)),
    }
}

struct ReplaceVisitor {
    search_id: i32,
    rep: Type,
}

impl ReplaceVisitor {
    fn new(search_id: &i32, rep: &Type) -> Self {
        ReplaceVisitor {
            search_id: *search_id,
            rep: rep.to_owned(),
        }
    }
}

impl Visitor for ReplaceVisitor {
    fn visit_type(&mut self, t: &mut Type) {
        if let TypeKind::Var(tvar) = &t.kind {
            if tvar.id == self.search_id {
                t.kind = self.rep.kind.to_owned();
                t.mutable = self.rep.mutable;
            }
        }
    }
}

fn replace_tvar(t: &mut Type, search_id: &i32, rep: &Type) {
    let mut rep_visitor = ReplaceVisitor::new(search_id, rep);
    rep_visitor.visit_children(t);
}