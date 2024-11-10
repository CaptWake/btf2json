//! Types and functions to work with ISF types.
//!
//! Only code that should be affected by a change in the ISF spec for types.
//!
//! The reoccurring pattern in this module is:
//!
//! - there is a Rust type that corresponds to a definition in the ISF JSON
//!   schema,
//! - the serialized form of an instance of this Rust type will conform to the
//!   schema definition,
//! - the types can only be build from a corresponding "construction context"
//!   type that entails what is sufficient information to construct the type.
//!
//! Besides the main types there are also some helper types that aid with
//! the construction of the main type. The order is:
//!
//! - helper types and their impls,
//! - main type,
//! - main type impls,
//! - construction context type,
//! - conversion code.

use crate::btf;

use std::collections::{BTreeMap, HashMap};
use std::convert::From;

use anyhow::{bail, Context, Error, Result};
use serde::{Deserialize, Serialize};

/// ISF `type_descriptor`.
#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
#[serde(tag = "kind")]
pub enum TypeDescr {
    #[serde(rename = "array")]
    Array { count: u64, subtype: Box<TypeDescr> },
    #[serde(rename = "base")]
    Base { name: String },
    #[serde(rename = "bitfield")]
    Bitfield {
        bit_position: u8,
        bit_length: u8,
        #[serde(rename = "type")]
        type_: Box<TypeDescr>,
    },
    #[serde(rename = "enum")]
    Enum { name: String },
    #[serde(rename = "function")]
    Function,
    #[serde(rename = "pointer")]
    Pointer { subtype: Box<TypeDescr> },
    #[serde(rename = "struct")]
    Struct { name: String },
    #[serde(rename = "union")]
    Union { name: String },
}

impl TypeDescr {
    pub fn new_void() -> Self {
        Self::Base { name: String::from("void") }
    }

    pub fn name(&self) -> Option<&String> {
        match self {
            Self::Base { name } => Some(name),
            Self::Enum { name } => Some(name),
            Self::Union { name } => Some(name),
            Self::Struct { name } => Some(name),
            _ => None,
        }
    }

    pub fn kind(&self) -> Option<&str> {
        match self {
            Self::Base { name: _ } => Some(""),
            Self::Enum { name: _ } => Some("enum"),
            Self::Union { name: _ } => Some("union"),
            Self::Struct { name: _ } => Some("struct"),
            _ => None,
        }
    }
    pub fn resolve(&self) -> &Self {
        let mut tmp = self;
        loop {
            match tmp {
                Self::Base { name: _ }
                | Self::Enum { name: _ }
                | Self::Union { name: _ }
                | Self::Function
                | Self::Struct { name: _ } => return tmp,
                Self::Array { count: _, subtype } => tmp = subtype,
                Self::Pointer { subtype } => tmp = subtype,
                Self::Bitfield {
                    bit_length: _,
                    bit_position: _,
                    type_: t,
                } => tmp = t,
            }
        }
    }
}

struct TypeDescrConstructionCtx<'a> {
    ufctx: &'a UserFieldConstructionCtx<'a, 'a>,
    rt: btf::ResolvedType,
    name: String,
    handle_bitfield: bool,
}

impl From<TypeDescrConstructionCtx<'_>> for TypeDescr {
    fn from(mut ctx: TypeDescrConstructionCtx<'_>) -> Self {
        log::trace!(
            "[{}::{}] path {:?},",
            ctx.ufctx.uctx.basic_ctx.tx.id,
            &ctx.ufctx.m.name(ctx.ufctx.uctx.basic_ctx.btf),
            ctx.rt.path
        );
        match ctx.rt.path.pop_node() {
            Some(btf::ResolutionPathNode::Pointer) => Self::Pointer {
                subtype: Box::new(TypeDescr::from(ctx)),
            },
            Some(btf::ResolutionPathNode::Array(nelem)) => Self::Array {
                count: nelem,
                subtype: Box::new(TypeDescr::from(ctx)),
            },
            Some(btf::ResolutionPathNode::Typedef(_)) => TypeDescr::from(ctx),
            None => {
                if ctx.ufctx.m.is_bitfield() && ctx.handle_bitfield {
                    let bfinfo = ctx.ufctx.m.bitfield_info().unwrap();
                    ctx.handle_bitfield = false;

                    Self::Bitfield {
                        bit_position: bfinfo.position,
                        bit_length: bfinfo.length,
                        type_: Box::new(TypeDescr::from(ctx)),
                    }
                } else if ctx.rt.tx.t.is_union() {
                    Self::Union { name: ctx.name }
                } else if ctx.rt.tx.t.is_struct() {
                    Self::Struct { name: ctx.name }
                } else if ctx.rt.tx.t.is_fwd() {
                    let kind = if ctx.rt.tx.t.is_fwd_struct() {
                        "struct"
                    } else {
                        "union"
                    };
                    log::info!(
                        "[{}::{}] `{} {}` from fwd declaration will likely not be present",
                        ctx.ufctx.uctx.basic_ctx.tx.id,
                        &ctx.ufctx.m.name(ctx.ufctx.uctx.basic_ctx.btf),
                        kind,
                        &ctx.name
                    );
                    if ctx.rt.tx.t.is_fwd_struct() {
                        Self::Struct { name: ctx.name }
                    } else {
                        Self::Union { name: ctx.name }
                    }
                } else if ctx.rt.tx.t.is_enum() {
                    Self::Enum { name: ctx.name }
                } else if ctx.rt.tx.t.is_base() {
                    Self::Base { name: ctx.name }
                } else if ctx.rt.tx.t.is_func() {
                    Self::Function
                } else {
                    panic!("Unable to construct type descriptor: res type {:?}", ctx.rt.tx)
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub enum BaseKind {
    #[serde(rename = "void")]
    Void,
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "char")]
    Char,
    #[serde(rename = "bool")]
    Bool,
}

impl TryFrom<&btf::Type> for BaseKind {
    type Error = Error;

    fn try_from(type_: &btf::Type) -> Result<Self> {
        match &type_.t {
            btf_rs::Type::Void => Ok(Self::Void),
            btf_rs::Type::Float(_) => Ok(Self::Float),
            btf_rs::Type::Int(i) => {
                if i.is_char() {
                    Ok(Self::Char)
                } else if i.is_bool() {
                    Ok(Self::Bool)
                } else {
                    Ok(Self::Int)
                }
            }
            _ => bail!(
                "Type {:?} cannot be converted to an ISF base type",
                type_
            ),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum Endian {
    #[serde(rename = "big")]
    Big,
    #[serde(rename = "little")]
    Little,
}

impl From<&btf::Endian> for Endian {
    fn from(endian: &btf::Endian) -> Self {
        match endian {
            btf::Endian::Big => Self::Big,
            btf::Endian::Little => Self::Little,
        }
    }
}

/// ISF `element_base_type`.
#[derive(Serialize)]
pub struct Base {
    size: u8,
    signed: bool,
    kind: BaseKind,
    endian: Endian,
}

impl Base {
    pub fn new_pointer(endian: Endian) -> Self {
        Self {
            // TODO: distinguish between 64 and 32 bit
            size: 8,
            signed: false,
            kind: BaseKind::Int,
            endian,
        }
    }
}

/// Argument for constructing an ISF base type.
pub struct BaseConstructionCtx<'a> {
    pub btf: &'a btf::Btf,
    pub tx: btf::TypeEx,
}

impl BaseConstructionCtx<'_> {
    pub fn construct(self) -> Base {
        self.into()
    }
}

impl From<BaseConstructionCtx<'_>> for Base {
    fn from(ctx: BaseConstructionCtx) -> Self {
        Base {
            size: ctx.tx.t.size().unwrap_or(0) as u8,
            signed: ctx.tx.t.signed().unwrap(),
            kind: (&ctx.tx.t)
                .try_into()
                .expect("Attempt to construct Volatility base type from invalid BTF type"),
            endian: (&ctx.btf.endian).into(),
        }
    }
}

#[derive(Serialize, PartialEq, Eq)]
pub enum UserKind {
    #[serde(rename = "struct")]
    Struct,
    #[serde(rename = "union")]
    Union,
    #[serde(rename = "class")]
    Class,
}

impl UserKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Class => "class",
        }
    }
}

impl TryFrom<&btf::Type> for UserKind {
    type Error = Error;

    fn try_from(t: &btf::Type) -> Result<Self> {
        if t.is_struct() {
            Ok(UserKind::Struct)
        } else if t.is_union() {
            Ok(UserKind::Union)
        } else {
            bail!("Type {:?} is not an ISF user type.", t)
        }
    }
}

/// ISF `field`.
#[derive(Serialize)]
pub struct UserField {
    #[serde(rename = "type")]
    pub t: TypeDescr,
    offset: u64,
    #[serde(rename = "anonymous")]
    anon: bool,
}

struct UserFieldConstructionCtx<'a, 'b> {
    uctx: &'a UserConstructionCtx<'b>,
    m: btf::Member<'b>,
}

impl From<UserFieldConstructionCtx<'_, '_>> for UserField {
    fn from(ctx: UserFieldConstructionCtx) -> Self {
        let rt = ctx
            .uctx
            .basic_ctx
            .btf
            .resolve_type_chain(ctx.m.get_tx(ctx.uctx.basic_ctx.btf));
        let name = rt.name(ctx.uctx.basic_ctx.btf);

        UserField {
            t: TypeDescr::from(TypeDescrConstructionCtx {
                ufctx: &ctx,
                rt,
                name,
                handle_bitfield: true,
            }),
            offset: ctx.m.byte_offset(),
            anon: ctx.m.is_anon(),
        }
    }
}

#[derive(Serialize)]
pub struct UserFields(HashMap<String, UserField>);

impl UserFields {
    pub fn iter(&self) -> impl Iterator<Item = (&String, &UserField)> {
        self.0.iter()
    }
}

impl TryFrom<&UserConstructionCtx<'_>> for UserFields {
    type Error = Error;

    fn try_from(ctx: &UserConstructionCtx) -> Result<Self> {
        let members = ctx
            .basic_ctx
            .tx
            .t
            .as_has_members()
            .context(
                "Cannot construct ISF user type from BTF type without members.",
            )?
            .members();

        Ok(Self(
            members
                .into_iter()
                .map(|m| {
                    (
                        m.name(ctx.basic_ctx.btf),
                        UserField::from(UserFieldConstructionCtx {
                            uctx: ctx,
                            m,
                        }),
                    )
                })
                .collect(),
        ))
    }
}

/// ISF `element_user_type`.
#[derive(Serialize)]
pub struct User {
    pub kind: UserKind,
    size: u64,
    pub fields: UserFields,
}

pub struct UserConstructionCtx<'a> {
    pub basic_ctx: BaseConstructionCtx<'a>,
    pub typedefs: &'a btf::Typedefs,
}

impl From<UserConstructionCtx<'_>> for User {
    fn from(ctx: UserConstructionCtx) -> Self {
        User {
            kind: UserKind::try_from(&ctx.basic_ctx.tx.t).expect(
                "Attempt to construct ISF user type from invalid BTF type.",
            ),
            size: ctx.basic_ctx.tx.t.size().unwrap() as u64,
            fields: UserFields::try_from(&ctx)
                .expect("Failed to construct user type."),
        }
    }
}

/// ISF `element_enum`.
#[derive(Serialize)]
pub struct Enum {
    size: u8,
    base: String,
    constants: HashMap<String, i128>,
}

pub struct EnumConstructionCtx<'a> {
    pub basic_ctx: BaseConstructionCtx<'a>,
    pub base_types: &'a BTreeMap<String, Base>,
}

impl From<EnumConstructionCtx<'_>> for Enum {
    fn from(ctx: EnumConstructionCtx) -> Self {
        let size = ctx.basic_ctx.tx.t.size().unwrap() as u8;
        let signed = ctx.basic_ctx.tx.t.signed().unwrap();

        let base = ctx
            .base_types
            .iter()
            .find(|(_, basic_type)| {
                basic_type.size == size
                    && basic_type.signed == signed
                    && basic_type.kind == BaseKind::Int
            })
            .map(|(name, _)| name.clone())
            .expect("Cannot find name for base type of enum.");

        Enum {
            size: ctx.basic_ctx.tx.t.size().unwrap() as u8,
            base,
            constants: ctx
                .basic_ctx
                .tx
                .t
                .as_enum()
                .unwrap()
                .variants(ctx.basic_ctx.btf)
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}
