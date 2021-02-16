// config.rs
//
// Copyright 2020
//      Valentin Gatien-Baron,
//      Raphaël Gomès <rgomes@octobus.net>
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

use super::layer;
use crate::config::layer::{
    ConfigError, ConfigLayer, ConfigParseError, ConfigValue,
};
use crate::utils::files::get_bytes_from_os_str;
use format_bytes::{write_bytes, DisplayBytes};
use std::env;
use std::path::{Path, PathBuf};
use std::str;

use crate::errors::{HgResultExt, IoResultExt};

/// Holds the config values for the current repository
/// TODO update this docstring once we support more sources
pub struct Config {
    layers: Vec<layer::ConfigLayer>,
}

impl DisplayBytes for Config {
    fn display_bytes(
        &self,
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        for (index, layer) in self.layers.iter().rev().enumerate() {
            write_bytes!(
                out,
                b"==== Layer {} (trusted: {}) ====\n{}",
                index,
                if layer.trusted {
                    &b"yes"[..]
                } else {
                    &b"no"[..]
                },
                layer
            )?;
        }
        Ok(())
    }
}

pub enum ConfigSource {
    /// Absolute path to a config file
    AbsPath(PathBuf),
    /// Already parsed (from the CLI, env, Python resources, etc.)
    Parsed(layer::ConfigLayer),
}

pub fn parse_bool(v: &[u8]) -> Option<bool> {
    match v.to_ascii_lowercase().as_slice() {
        b"1" | b"yes" | b"true" | b"on" | b"always" => Some(true),
        b"0" | b"no" | b"false" | b"off" | b"never" => Some(false),
        _ => None,
    }
}

pub fn parse_byte_size(value: &[u8]) -> Option<u64> {
    let value = str::from_utf8(value).ok()?.to_ascii_lowercase();
    const UNITS: &[(&str, u64)] = &[
        ("g", 1 << 30),
        ("gb", 1 << 30),
        ("m", 1 << 20),
        ("mb", 1 << 20),
        ("k", 1 << 10),
        ("kb", 1 << 10),
        ("b", 1 << 0), // Needs to be last
    ];
    for &(unit, multiplier) in UNITS {
        // TODO: use `value.strip_suffix(unit)` when we require Rust 1.45+
        if value.ends_with(unit) {
            let value_before_unit = &value[..value.len() - unit.len()];
            let float: f64 = value_before_unit.trim().parse().ok()?;
            if float >= 0.0 {
                return Some((float * multiplier as f64).round() as u64);
            } else {
                return None;
            }
        }
    }
    value.parse().ok()
}

impl Config {
    /// Load system and user configuration from various files.
    ///
    /// This is also affected by some environment variables.
    pub fn load(
        cli_config_args: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<Self, ConfigError> {
        let mut config = Self { layers: Vec::new() };
        let opt_rc_path = env::var_os("HGRCPATH");
        // HGRCPATH replaces system config
        if opt_rc_path.is_none() {
            config.add_system_config()?
        }
        config.add_for_environment_variable("EDITOR", b"ui", b"editor");
        config.add_for_environment_variable("VISUAL", b"ui", b"editor");
        config.add_for_environment_variable("PAGER", b"pager", b"pager");
        // HGRCPATH replaces user config
        if opt_rc_path.is_none() {
            config.add_user_config()?
        }
        if let Some(rc_path) = &opt_rc_path {
            for path in env::split_paths(rc_path) {
                if !path.as_os_str().is_empty() {
                    if path.is_dir() {
                        config.add_trusted_dir(&path)?
                    } else {
                        config.add_trusted_file(&path)?
                    }
                }
            }
        }
        if let Some(layer) = ConfigLayer::parse_cli_args(cli_config_args)? {
            config.layers.push(layer)
        }
        Ok(config)
    }

    fn add_trusted_dir(&mut self, path: &Path) -> Result<(), ConfigError> {
        if let Some(entries) = std::fs::read_dir(path)
            .for_file(path)
            .io_not_found_as_none()?
        {
            for entry in entries {
                let file_path = entry.for_file(path)?.path();
                if file_path.extension() == Some(std::ffi::OsStr::new("rc")) {
                    self.add_trusted_file(&file_path)?
                }
            }
        }
        Ok(())
    }

    fn add_trusted_file(&mut self, path: &Path) -> Result<(), ConfigError> {
        if let Some(data) =
            std::fs::read(path).for_file(path).io_not_found_as_none()?
        {
            self.layers.extend(ConfigLayer::parse(path, &data)?)
        }
        Ok(())
    }

    fn add_for_environment_variable(
        &mut self,
        var: &str,
        section: &[u8],
        key: &[u8],
    ) {
        if let Some(value) = env::var_os(var) {
            let origin = layer::ConfigOrigin::Environment(var.into());
            let mut layer = ConfigLayer::new(origin);
            layer.add(
                section.to_owned(),
                key.to_owned(),
                get_bytes_from_os_str(value),
                None,
            );
            self.layers.push(layer)
        }
    }

    #[cfg(unix)] // TODO: other platforms
    fn add_system_config(&mut self) -> Result<(), ConfigError> {
        let mut add_for_prefix = |prefix: &Path| -> Result<(), ConfigError> {
            let etc = prefix.join("etc").join("mercurial");
            self.add_trusted_file(&etc.join("hgrc"))?;
            self.add_trusted_dir(&etc.join("hgrc.d"))
        };
        let root = Path::new("/");
        // TODO: use `std::env::args_os().next().unwrap()` a.k.a. argv[0]
        // instead? TODO: can this be a relative path?
        let hg = crate::utils::current_exe()?;
        // TODO: this order (per-installation then per-system) matches
        // `systemrcpath()` in `mercurial/scmposix.py`, but
        // `mercurial/helptext/config.txt` suggests it should be reversed
        if let Some(installation_prefix) = hg.parent().and_then(Path::parent) {
            if installation_prefix != root {
                add_for_prefix(&installation_prefix)?
            }
        }
        add_for_prefix(root)?;
        Ok(())
    }

    #[cfg(unix)] // TODO: other plateforms
    fn add_user_config(&mut self) -> Result<(), ConfigError> {
        let opt_home = home::home_dir();
        if let Some(home) = &opt_home {
            self.add_trusted_file(&home.join(".hgrc"))?
        }
        let darwin = cfg!(any(target_os = "macos", target_os = "ios"));
        if !darwin {
            if let Some(config_home) = env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| opt_home.map(|home| home.join(".config")))
            {
                self.add_trusted_file(&config_home.join("hg").join("hgrc"))?
            }
        }
        Ok(())
    }

    /// Loads in order, which means that the precedence is the same
    /// as the order of `sources`.
    pub fn load_from_explicit_sources(
        sources: Vec<ConfigSource>,
    ) -> Result<Self, ConfigError> {
        let mut layers = vec![];

        for source in sources.into_iter() {
            match source {
                ConfigSource::Parsed(c) => layers.push(c),
                ConfigSource::AbsPath(c) => {
                    // TODO check if it should be trusted
                    // mercurial/ui.py:427
                    let data = match std::fs::read(&c) {
                        Err(_) => continue, // same as the python code
                        Ok(data) => data,
                    };
                    layers.extend(ConfigLayer::parse(&c, &data)?)
                }
            }
        }

        Ok(Config { layers })
    }

    /// Loads the per-repository config into a new `Config` which is combined
    /// with `self`.
    pub(crate) fn combine_with_repo(
        &self,
        repo_config_files: &[PathBuf],
    ) -> Result<Self, ConfigError> {
        let (cli_layers, other_layers) = self
            .layers
            .iter()
            .cloned()
            .partition(ConfigLayer::is_from_command_line);

        let mut repo_config = Self {
            layers: other_layers,
        };
        for path in repo_config_files {
            // TODO: check if this file should be trusted:
            // `mercurial/ui.py:427`
            repo_config.add_trusted_file(path)?;
        }
        repo_config.layers.extend(cli_layers);
        Ok(repo_config)
    }

    fn get_parse<'config, T: 'config>(
        &'config self,
        section: &[u8],
        item: &[u8],
        parse: impl Fn(&'config [u8]) -> Option<T>,
    ) -> Result<Option<T>, ConfigParseError> {
        match self.get_inner(&section, &item) {
            Some((layer, v)) => match parse(&v.bytes) {
                Some(b) => Ok(Some(b)),
                None => Err(ConfigParseError {
                    origin: layer.origin.to_owned(),
                    line: v.line,
                    bytes: v.bytes.to_owned(),
                }),
            },
            None => Ok(None),
        }
    }

    /// Returns an `Err` if the first value found is not a valid UTF-8 string.
    /// Otherwise, returns an `Ok(value)` if found, or `None`.
    pub fn get_str(
        &self,
        section: &[u8],
        item: &[u8],
    ) -> Result<Option<&str>, ConfigParseError> {
        self.get_parse(section, item, |value| str::from_utf8(value).ok())
    }

    /// Returns an `Err` if the first value found is not a valid unsigned
    /// integer. Otherwise, returns an `Ok(value)` if found, or `None`.
    pub fn get_u32(
        &self,
        section: &[u8],
        item: &[u8],
    ) -> Result<Option<u32>, ConfigParseError> {
        self.get_parse(section, item, |value| {
            str::from_utf8(value).ok()?.parse().ok()
        })
    }

    /// Returns an `Err` if the first value found is not a valid file size
    /// value such as `30` (default unit is bytes), `7 MB`, or `42.5 kb`.
    /// Otherwise, returns an `Ok(value_in_bytes)` if found, or `None`.
    pub fn get_byte_size(
        &self,
        section: &[u8],
        item: &[u8],
    ) -> Result<Option<u64>, ConfigParseError> {
        self.get_parse(section, item, parse_byte_size)
    }

    /// Returns an `Err` if the first value found is not a valid boolean.
    /// Otherwise, returns an `Ok(option)`, where `option` is the boolean if
    /// found, or `None`.
    pub fn get_option(
        &self,
        section: &[u8],
        item: &[u8],
    ) -> Result<Option<bool>, ConfigParseError> {
        self.get_parse(section, item, parse_bool)
    }

    /// Returns the corresponding boolean in the config. Returns `Ok(false)`
    /// if the value is not found, an `Err` if it's not a valid boolean.
    pub fn get_bool(
        &self,
        section: &[u8],
        item: &[u8],
    ) -> Result<bool, ConfigError> {
        Ok(self.get_option(section, item)?.unwrap_or(false))
    }

    /// Returns the raw value bytes of the first one found, or `None`.
    pub fn get(&self, section: &[u8], item: &[u8]) -> Option<&[u8]> {
        self.get_inner(section, item)
            .map(|(_, value)| value.bytes.as_ref())
    }

    /// Returns the layer and the value of the first one found, or `None`.
    fn get_inner(
        &self,
        section: &[u8],
        item: &[u8],
    ) -> Option<(&ConfigLayer, &ConfigValue)> {
        for layer in self.layers.iter().rev() {
            if !layer.trusted {
                continue;
            }
            if let Some(v) = layer.get(&section, &item) {
                return Some((&layer, v));
            }
        }
        None
    }

    /// Get raw values bytes from all layers (even untrusted ones) in order
    /// of precedence.
    #[cfg(test)]
    fn get_all(&self, section: &[u8], item: &[u8]) -> Vec<&[u8]> {
        let mut res = vec![];
        for layer in self.layers.iter().rev() {
            if let Some(v) = layer.get(&section, &item) {
                res.push(v.bytes.as_ref());
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_include_layer_ordering() {
        let tmpdir = tempfile::tempdir().unwrap();
        let tmpdir_path = tmpdir.path();
        let mut included_file =
            File::create(&tmpdir_path.join("included.rc")).unwrap();

        included_file.write_all(b"[section]\nitem=value1").unwrap();
        let base_config_path = tmpdir_path.join("base.rc");
        let mut config_file = File::create(&base_config_path).unwrap();
        let data =
            b"[section]\nitem=value0\n%include included.rc\nitem=value2\n\
              [section2]\ncount = 4\nsize = 1.5 KB\nnot-count = 1.5\nnot-size = 1 ub";
        config_file.write_all(data).unwrap();

        let sources = vec![ConfigSource::AbsPath(base_config_path)];
        let config = Config::load_from_explicit_sources(sources)
            .expect("expected valid config");

        let (_, value) = config.get_inner(b"section", b"item").unwrap();
        assert_eq!(
            value,
            &ConfigValue {
                bytes: b"value2".to_vec(),
                line: Some(4)
            }
        );

        let value = config.get(b"section", b"item").unwrap();
        assert_eq!(value, b"value2",);
        assert_eq!(
            config.get_all(b"section", b"item"),
            [b"value2", b"value1", b"value0"]
        );

        assert_eq!(config.get_u32(b"section2", b"count").unwrap(), Some(4));
        assert_eq!(
            config.get_byte_size(b"section2", b"size").unwrap(),
            Some(1024 + 512)
        );
        assert!(config.get_u32(b"section2", b"not-count").is_err());
        assert!(config.get_byte_size(b"section2", b"not-size").is_err());
    }
}
