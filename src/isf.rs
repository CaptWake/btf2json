//! Internal representation of ISF files.

use crate::metadata::Metadata;
use crate::GenerationContext;
use crate::{btf, v_symbols, v_types};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::convert::TryFrom;

use anyhow::{bail, Error, Result};
use serde::Serialize;

mod quirks {
    //! Manual adjustments that we have to make to the ISF file in order to meet
    //! Volatility's expectations.

    use super::*;
    use std::collections::btree_map::Entry;

    /// Adds a base type named "pointer" with the appropriate size and
    /// endianness.
    pub fn fixup_base(
        base_types: &mut BTreeMap<String, v_types::Base>,
        endian: &btf::Endian,
    ) {
        if let Entry::Vacant(ent) = base_types.entry(String::from("pointer")) {
            ent.insert(v_types::Base::new_pointer(endian.into()));
        }
    }
}

/// Representation of an ISF file.
#[derive(Serialize)]
pub struct Isf {
    metadata: Metadata,
    user_types: BTreeMap<String, v_types::User>,
    enums: BTreeMap<String, v_types::Enum>,
    // We cannot use a hash map here as the iteration order matters when
    // determining the name of the base type of an enum.
    base_types: BTreeMap<String, v_types::Base>,
    symbols: BTreeMap<String, v_symbols::Symbol>,
}

impl Isf {
    fn map_from_ids<F, G, T>(
        ids: &BTreeSet<btf::Id>,
        id_to_names: F,
        name_to_elem: G,
    ) -> BTreeMap<String, T>
    where
        F: Fn(btf::Id) -> Vec<String>,
        G: Fn(String, btf::Id) -> (String, T),
    {
        ids.iter()
            .flat_map(|id| {
                id_to_names(*id)
                    .into_iter()
                    .map(|name| name_to_elem(name, *id))
            })
            .collect()
    }
}

impl TryFrom<GenerationContext> for Isf {
    type Error = Error;

    /// Try to construct ISF file from gathered information.
    fn try_from(mut ctx: GenerationContext) -> Result<Isf> {
        let mut base_types = Isf::map_from_ids(
            &ctx.basic_ids,
            |id| ctx.btf.get_names_by_id(id, None).unwrap(),
            |name, id| {
                (
                    name,
                    v_types::Base::from(v_types::BaseConstructionCtx {
                        btf: &ctx.btf,
                        tx: btf::TypeEx {
                            t: ctx.btf.get_type_by_id(id).unwrap(),
                            id,
                        },
                    }),
                )
            },
        );
        quirks::fixup_base(&mut base_types, &ctx.btf.endian);

        Ok(Isf {
            metadata: ctx.mbuilder.take().unwrap().into(),
            user_types: Isf::map_from_ids(
                &ctx.user_ids,
                |id| ctx.btf.get_names_by_id(id, Some(&ctx.typedefs)).unwrap(),
                |name, id| {
                    (
                        name,
                        v_types::User::from(v_types::UserConstructionCtx {
                            basic_ctx: v_types::BaseConstructionCtx {
                                btf: &ctx.btf,
                                tx: btf::TypeEx {
                                    t: ctx.btf.get_type_by_id(id).unwrap(),
                                    id,
                                },
                            },
                            typedefs: &ctx.typedefs,
                        }),
                    )
                },
            ),
            enums: Isf::map_from_ids(
                &ctx.enum_ids,
                |id| ctx.btf.get_names_by_id(id, Some(&ctx.typedefs)).unwrap(),
                |name, id| {
                    (
                        name,
                        v_types::Enum::from(v_types::EnumConstructionCtx {
                            basic_ctx: v_types::BaseConstructionCtx {
                                btf: &ctx.btf,
                                tx: btf::TypeEx {
                                    t: ctx.btf.get_type_by_id(id).unwrap(),
                                    id,
                                },
                            },
                            base_types: &base_types,
                        }),
                    )
                },
            ),
            base_types,
            symbols: ctx
                .symbols
                .into_iter()
                .map(|(name, sym)| (name, sym.into()))
                .collect(),
        })
    }
}

impl Isf {
    /// Writes a valid ISF file to stdout.
    pub fn dump_stdout(&self) {
        log::debug!(
            "ISF elements: base {}, enum {}, user {}, symbol {}",
            &self.base_types.len(),
            &self.enums.len(),
            &self.user_types.len(),
            &self.symbols.len()
        );
        println!("{}", serde_json::to_string(&self).unwrap());
    }

    /// Verifies that all types referenced by fields of user types are defined.
    pub fn check_user_types(&self) -> Result<()> {
        let mut problematic_types: HashMap<String, Vec<&String>> =
            HashMap::new();
        let mut undefined_types: HashSet<String> = HashSet::new();

        for (name, ut) in self.user_types.iter() {
            let problematic_fields: Vec<&String> = ut
                .fields
                .iter()
                .filter_map(|(field_name, field)| {
                    if self.is_defined(&field.t) {
                        return None;
                    }
                    let rt = field.t.resolve();
                    let field_type_kind = rt.kind().unwrap();
                    let field_type_name = rt.name().unwrap();
                    log::warn!(
                        "[{} {}::{}] has undefined type `{} {}`",
                        ut.kind.as_str(),
                        name,
                        field_name,
                        field_type_kind,
                        field_type_name,
                    );
                    undefined_types.insert(format!(
                        "{} {}",
                        field_type_kind, field_type_name
                    ));
                    Some(field_name)
                })
                .collect();
            if !problematic_fields.is_empty() {
                problematic_types.insert(
                    format!("{} {}", ut.kind.as_str(), name.as_str()),
                    problematic_fields,
                );
            }
        }

        if problematic_types.is_empty() {
            log::debug!("All types referenced by user types are present");
            Ok(())
        } else {
            log::error!(
                "{} user types have fields that reference undefined types, {} unique types undefined, {} unique fields affected",
                problematic_types.len(),
                undefined_types.len(),
                problematic_types.iter().flat_map(|(_, v)| v.iter()).count(),
            );
            bail!("User type verification failed.")
        }
    }

    /// Tests if a type is defined in the ISF file.
    fn is_defined(&self, t: &v_types::TypeDescr) -> bool {
        let rt = t.resolve();
        match rt {
            v_types::TypeDescr::Base { name } => {
                self.base_types.contains_key(name)
            }
            v_types::TypeDescr::Enum { name } => self.enums.contains_key(name),
            v_types::TypeDescr::Union { name } => self
                .user_types
                .get(name)
                .is_some_and(|t| t.kind == v_types::UserKind::Union),
            v_types::TypeDescr::Struct { name } => self
                .user_types
                .get(name)
                .is_some_and(|t| t.kind == v_types::UserKind::Struct),
            v_types::TypeDescr::Function => true,
            _ => panic!("BUG: type descriptor resolution failed: {:?}", rt),
        }
    }

    /// Removes undefined types from symbols.
    ///
    /// As we rely on a database to determine the type of symbols there will be
    /// some cases where the referenced types are not defined in the BTF
    /// section. In that case we simply set the type of the symbol to "void".
    pub fn fix_symbol_types(&mut self) -> Result<()> {
        let mut problematic_symbols: HashSet<String> = HashSet::new();
        let mut missing_types: HashSet<&v_types::TypeDescr> = HashSet::new();

        for (name, sym) in self.symbols.iter() {
            if self.is_defined(&sym.t) {
                continue;
            }
            let rt = sym.t.resolve();
            log::warn!("Symbol {} references non-present type {:?}", name, rt);
            missing_types.insert(rt);
            problematic_symbols.insert(name.to_owned());
        }

        if problematic_symbols.is_empty() {
            log::debug!("All types referenced by symbols are present");
            Ok(())
        } else {
            log::error!(
                "{} symbols reference missing types, {} unique types are missing",
                problematic_symbols.len(),
                missing_types.len(),
            );
            // fixup symbols
            for sym_name in problematic_symbols.iter() {
                let sym = self.symbols.get_mut(sym_name).unwrap();
                sym.t = v_types::TypeDescr::new_void();
            }
            bail!("Symbol type verification failed.")
        }
    }
}
