// Types and type constructors
use crate::literal::Literal;

pub type ArenaType = usize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    pub instance: Option<ArenaType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Constructor {
    pub name: String,
    pub types: Vec<ArenaType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ref {
    pub name: String,
    // TODO: Add support for type args
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Function {
    pub params: Vec<ArenaType>,
    pub ret: ArenaType,
    pub type_params: Option<Vec<TypeParam>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Call {
    pub args: Vec<ArenaType>,
    pub ret: ArenaType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Union {
    pub types: Vec<ArenaType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tuple {
    pub types: Vec<ArenaType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Object {
    pub props: Vec<(String, ArenaType)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Variable(Variable),
    Constructor(Constructor),
    Ref(Ref),
    Literal(Literal),
    Function(Function),
    Union(Union),
    Tuple(Tuple),
    Object(Object),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Type {
    pub id: ArenaType,
    pub kind: TypeKind,
}

/// A type variable standing for an arbitrary type.
///
/// All type variables have a unique id, but names are
/// only assigned lazily, when required.

impl Type {
    pub fn new_variable(idx: ArenaType) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Variable(Variable { instance: None }),
        }
    }

    pub fn new_constructor(idx: ArenaType, name: &str, types: &[ArenaType]) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Constructor(Constructor {
                name: name.to_string(),
                types: types.to_vec(),
            }),
        }
    }

    // TODO: add support for type args
    pub fn new_type_ref(idx: ArenaType, name: &str) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Ref(Ref {
                name: name.to_string(),
            }),
        }
    }

    pub fn new_literal(idx: ArenaType, lit: &Literal) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Literal(lit.clone()),
        }
    }

    pub fn new_function(
        idx: ArenaType,
        param_types: &[ArenaType],
        ret_type: ArenaType,
        type_params: Option<&[TypeParam]>,
    ) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Function(Function {
                params: param_types.to_vec(),
                ret: ret_type,
                type_params: type_params.map(|x| x.to_vec()),
            }),
        }
    }

    pub fn new_union(idx: ArenaType, types: &[ArenaType]) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Union(Union {
                types: types.to_vec(),
            }),
        }
    }

    pub fn new_tuple(idx: ArenaType, types: &[ArenaType]) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Tuple(Tuple {
                types: types.to_vec(),
            }),
        }
    }

    pub fn new_object(idx: ArenaType, props: &[(String, ArenaType)]) -> Type {
        Type {
            id: idx,
            kind: TypeKind::Object(Object {
                props: props.to_vec(),
            }),
        }
    }

    pub fn set_instance(&mut self, instance: ArenaType) {
        match &mut self.kind {
            TypeKind::Variable(Variable {
                instance: ref mut inst,
                ..
            }) => {
                *inst = Some(instance);
            }
            _ => {
                unimplemented!()
            }
        }
    }

    pub fn as_string(&self, a: &Vec<Type>) -> String {
        match &self.kind {
            TypeKind::Variable(Variable {
                instance: Some(inst),
            }) => a[*inst].as_string(a),
            TypeKind::Variable(_) => format!("t{}", self.id),
            TypeKind::Constructor(con) => match con.types.len() {
                0 => con.name.clone(),
                2 => {
                    let l = a[con.types[0]].as_string(a);
                    let r = a[con.types[1]].as_string(a);
                    format!("({} {} {})", l, con.name, r)
                }
                _ => {
                    let mut coll = vec![];
                    for v in &con.types {
                        coll.push(a[*v].as_string(a));
                    }
                    format!("{} {}", con.name, coll.join(" "))
                }
            },
            TypeKind::Ref(Ref { name }) => name.clone(),
            TypeKind::Literal(lit) => lit.to_string(),
            TypeKind::Tuple(tuple) => {
                format!("[{}]", types_to_strings(a, &tuple.types).join(", "))
            }
            TypeKind::Object(object) => {
                let mut fields = vec![];
                for (k, v) in &object.props {
                    fields.push(format!("{}: {}", k, a[*v].as_string(a)));
                }
                format!("{{{}}}", fields.join(", "))
            }
            TypeKind::Function(func) => {
                let type_params = if let Some(type_params) = &func.type_params {
                    let type_params = type_params
                        .iter()
                        .map(|tp| tp.name.clone())
                        .collect::<Vec<_>>();
                    format!("<{}>", type_params.join(", "))
                } else {
                    "".to_string()
                };
                format!(
                    "{type_params}({}) => {}",
                    types_to_strings(a, &func.params).join(", "),
                    a[func.ret].as_string(a),
                )
            }
            TypeKind::Union(union) => types_to_strings(a, &union.types).join(" | "),
        }
    }
}

fn types_to_strings(a: &Vec<Type>, types: &[ArenaType]) -> Vec<String> {
    let mut strings = vec![];
    for v in types {
        strings.push(a[*v].as_string(a));
    }
    strings
}

/// A binary type constructor which builds function types
pub fn new_func_type(
    a: &mut Vec<Type>,
    params: &[ArenaType],
    ret: ArenaType,
    type_params: Option<&[TypeParam]>,
) -> ArenaType {
    let t = Type::new_function(a.len(), params, ret, type_params);
    a.push(t);
    a.len() - 1
}

pub fn new_union_type(a: &mut Vec<Type>, types: &[ArenaType]) -> ArenaType {
    let t = Type::new_union(a.len(), types);
    a.push(t);
    a.len() - 1
}

pub fn new_tuple_type(a: &mut Vec<Type>, types: &[ArenaType]) -> ArenaType {
    let t = Type::new_tuple(a.len(), types);
    a.push(t);
    a.len() - 1
}

pub fn new_object_type(a: &mut Vec<Type>, props: &[(String, ArenaType)]) -> ArenaType {
    let t = Type::new_object(a.len(), props);
    a.push(t);
    a.len() - 1
}

/// A binary type constructor which builds function types
pub fn new_var_type(a: &mut Vec<Type>) -> ArenaType {
    let t = Type::new_variable(a.len());
    a.push(t);
    a.len() - 1
}

/// A binary type constructor which builds function types
pub fn new_constructor(a: &mut Vec<Type>, name: &str, types: &[ArenaType]) -> ArenaType {
    let t = Type::new_constructor(a.len(), name, types);
    a.push(t);
    a.len() - 1
}

// TODO: add support for type args
pub fn new_type_ref(a: &mut Vec<Type>, name: &str) -> ArenaType {
    let t = Type::new_type_ref(a.len(), name);
    a.push(t);
    a.len() - 1
}

pub fn new_lit_type(a: &mut Vec<Type>, lit: &Literal) -> ArenaType {
    let t = Type::new_literal(a.len(), lit);
    a.push(t);
    a.len() - 1
}

pub fn new_num_lit_type(a: &mut Vec<Type>, value: &str) -> ArenaType {
    let t = Type::new_literal(a.len(), &Literal::Number(value.to_string()));
    a.push(t);
    a.len() - 1
}

pub fn new_str_lit_type(a: &mut Vec<Type>, value: &str) -> ArenaType {
    let t = Type::new_literal(a.len(), &Literal::String(value.to_string()));
    a.push(t);
    a.len() - 1
}

pub fn new_bool_lit_type(a: &mut Vec<Type>, value: bool) -> ArenaType {
    let t = Type::new_literal(a.len(), &Literal::Boolean(value));
    a.push(t);
    a.len() - 1
}

//impl fmt::Debug for Type {
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        match self {
//            write!(f, "TypeVariable(id = {})", self.id)
//            write!(f, "TypeOperator(name, )", self.id)
//        }
//    }
//}
