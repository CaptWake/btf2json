//! Command-line interface.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "btf2json", author = "Valentin Obst")]
/// Generate Volatility 3 ISF files from BTF type information.
pub struct Cli {
    #[clap(long = "btf")]
    /// BTF file for obtaining type information (can also be a kernel image).
    pub btf: Option<PathBuf>,
    #[clap(long = "map")]
    /// System.map file for obtaining symbol names and addresses.
    pub map: Option<PathBuf>,
    #[clap(long = "banner")]
    /// Linux banner.
    ///
    /// Mandatory if using a BTF file for type information. Takes precedence
    /// over all other possible sources of banner information.
    pub banner: Option<String>,
    #[clap(long = "version")]
    /// Print btf2json version.
    pub version: bool,
    #[clap(long = "verbose")]
    /// Display debug output.
    pub verbose: bool,
    #[clap(long = "debug")]
    /// Display more debug output.
    pub debug: bool,
    #[clap(long = "image")]
    /// Memory image to extract type and/or symbol information from (not
    /// implemented).
    pub image: Option<PathBuf>,
}
