//! Types and functions to generate ISF symbol entries.
//!
//! Only code that should be affected by a change in the ISF spec for symbols.

use crate::symbols;
use crate::v_types;

use serde::Serialize;

#[derive(Serialize)]
#[allow(dead_code)]
enum Linkage {
    Global,
    Static,
}

/// Represents an ISF symbol.
#[derive(Serialize)]
pub struct Symbol {
    pub address: u64,
    #[serde(rename = "type")]
    pub t: v_types::TypeDescr,
    #[serde(skip_serializing_if = "Option::is_none")]
    linkage: Option<Linkage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    constant_data: Option<String>,
}

impl From<symbols::Symbol> for Symbol {
    fn from(mut sym: symbols::Symbol) -> Self {
        Symbol {
            address: sym.address(),
            t: sym
                .r#type()
                .map(|t| match serde_json::from_str::<v_types::TypeDescr>(&t) {
                    Ok(t) => t,
                    Err(e) => {
                        panic!("Symbol type had invalid format: {}: {}", e, t)
                    }
                })
                .unwrap_or(v_types::TypeDescr::Base {
                    name: String::from("void"),
                }),
            linkage: None,
            constant_data: sym.constant_data(),
        }
    }
}
