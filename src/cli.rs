//! Command-line interface.

use std::path::PathBuf;

use clap::Parser;
use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum Architecture {
    #[value(name = "x86_64")]
    X86_64,
    #[value(name = "arm64")]
    Arm64,
}

impl Default for Architecture {
    fn default() -> Self {
        Architecture::X86_64
    }
}

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
    /// Define the architecture of the system for which the ISF is generated.
    #[clap(long = "arch", value_enum, default_value_t = Architecture::default())]
    pub arch: Architecture,
    /// Memory image to extract type and/or symbol information from (not
    /// implemented).
    #[clap(long = "image")]
    pub image: Option<PathBuf>,
}
