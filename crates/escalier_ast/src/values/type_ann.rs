use crate::types::Type;
use crate::values::common::{SourceLocation, Span};
use crate::values::expr::Expr;
use crate::values::ident::Ident;
use crate::values::keyword::Keyword;
use crate::values::lit::Lit;
use crate::values::pattern::Pattern;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAnnFnParam {
    pub pat: Pattern,
    pub type_ann: TypeAnn,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LamType {
    pub params: Vec<TypeAnnFnParam>,
    pub ret: Box<TypeAnn>,
    pub type_params: Option<Vec<TypeParam>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeywordType {
    pub keyword: Keyword,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRef {
    pub name: String,
    // TODO: drop the Option
    pub type_args: Option<Vec<TypeAnn>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectType {
    // TODO: update this to support indexers and callables as well.
    pub elems: Vec<TObjElem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TObjElem {
    // Call(TLam),
    // Constructor(TLam),
    Index(TIndex),
    Prop(TProp),
    // Getter
    // Setter
    // RestSpread - we can use this instead of converting {a, ...x} to {a} & tvar
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TProp {
    pub loc: SourceLocation,
    pub span: Span,
    pub name: String,
    pub optional: bool,
    pub mutable: bool,
    pub type_ann: Box<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TIndexKey {
    pub name: String,
    pub type_ann: Box<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TIndex {
    pub loc: SourceLocation,
    pub span: Span,
    // TODO: update this to only allow `<ident>: string` or `<ident>: number`
    pub key: Box<TypeAnnFnParam>,
    pub mutable: bool,
    pub type_ann: Box<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionType {
    pub types: Vec<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntersectionType {
    pub types: Vec<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TupleType {
    pub types: Vec<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayType {
    pub elem_type: Box<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyOfType {
    pub type_ann: Box<TypeAnn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryType {
    // TypeScript only supports typeof on (qualified) identifiers.
    // We could modify the parser if we wanted to support taking
    // the type of arbitrary expressions.
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedAccessType {
    pub obj_type: Box<TypeAnn>,
    pub index_type: Box<TypeAnn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedType {
    pub type_param: TypeParam, // default is always None for MappedType
    pub optional: Option<TMappedTypeChange>,
    pub mutable: Option<TMappedTypeChange>,
    pub type_ann: Box<TypeAnn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TMappedTypeChange {
    pub span: Span,
    pub change: TMappedTypeChangeProp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TMappedTypeChangeProp {
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalType {
    pub check_type: Box<TypeAnn>,
    pub extends_type: Box<TypeAnn>,
    pub true_type: Box<TypeAnn>,
    pub false_type: Box<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutableType {
    pub type_ann: Box<TypeAnn>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnnKind {
    Lam(LamType),
    Lit(Lit),
    Keyword(KeywordType),
    Object(ObjectType),
    TypeRef(TypeRef),
    Union(UnionType),
    Intersection(IntersectionType),
    Tuple(TupleType),
    Array(ArrayType), // T[]
    KeyOf(KeyOfType), // keyof
    Query(QueryType), // typeof
    IndexedAccess(IndexedAccessType),
    Mapped(MappedType),
    Conditional(ConditionalType),
    Mutable(MutableType),
    Infer(InferType),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeAnn {
    pub kind: TypeAnnKind,
    pub loc: SourceLocation,
    pub span: Span,
    pub inferred_type: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    pub span: Span,
    pub name: Ident,
    pub constraint: Option<Box<TypeAnn>>,
    pub default: Option<Box<TypeAnn>>,
}