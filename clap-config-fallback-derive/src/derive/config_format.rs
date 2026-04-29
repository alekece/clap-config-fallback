use darling::FromMeta;

/// `format` attribute for configuration files, allowing explicit specification of the format or
/// automatic detection based on file extension.
#[derive(Copy, Clone, Default, FromMeta)]
pub enum ConfigFormat {
    /// Explicitly specify that the configuration file is in TOML format.
    Toml,
    /// Explicitly specify that the configuration file is in YAML format.
    Yaml,
    /// Explicitly specify that the configuration file is in JSON format.
    Json,
    /// Indicates that the configuration format should be determined automatically based on the file
    /// extension of the path field, if any.
    #[default]
    Auto,
}
