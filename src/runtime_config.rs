use anyhow::Result;

/// Holds the runtime configuration for the program.
/// Used to turn features on/off.
pub struct Config {
    pub enable_height_map: bool,
    pub input_folder: String,
    pub output_folder: String,
    pub verbose: bool,
}

impl Config {
    /// Read command line arguments and flags to generate the runtime configuration.
    pub fn new() -> Result<Self> {
        const VERBOSE: (&str, &str, &str) = (
            "verbose",
            "v",
            "if set, all logging output is printed. \
                Side-effect: any multi-threading that uses logging is disabled. \
                Note that there is always a log-file in the output folder with the full log",
        );
        const ENABLE_HEIGHT_MAP: (&str, &str) = (
            "enable-height-map",
            "enables generating of height maps derived from the normal maps",
        );
        const INPUT: (&str, &str, &str, &str) = (
            "input",
            "i",
            "the path to the input folder with the extracted files from game archives",
            "input",
        );
        const OUTPUT: (&str, &str, &str, &str) = (
            "output",
            "o",
            "the path to the output folder where the converted files are stored",
            "output",
        );
        use clap::{App, Arg};
        let matches = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(
                Arg::with_name(VERBOSE.0)
                    .short(VERBOSE.1)
                    .long(VERBOSE.0)
                    .help(VERBOSE.2)
                    .takes_value(false),
            )
            .arg(
                Arg::with_name(ENABLE_HEIGHT_MAP.0)
                    .long(ENABLE_HEIGHT_MAP.0)
                    .help(ENABLE_HEIGHT_MAP.1)
                    .takes_value(false),
            )
            .arg(
                Arg::with_name(INPUT.0)
                    .short(INPUT.1)
                    .long(INPUT.0)
                    .help(INPUT.2)
                    .default_value(INPUT.3)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name(OUTPUT.0)
                    .short(OUTPUT.1)
                    .long(OUTPUT.0)
                    .help(OUTPUT.2)
                    .default_value(OUTPUT.3)
                    .takes_value(true),
            )
            .get_matches();

        Ok(Self {
            enable_height_map: matches.is_present(ENABLE_HEIGHT_MAP.0),
            input_folder: matches.value_of(INPUT.0).unwrap().to_string(),
            output_folder: matches.value_of(OUTPUT.0).unwrap().to_string(),
            verbose: matches.is_present(VERBOSE.0),
        })
    }
}
