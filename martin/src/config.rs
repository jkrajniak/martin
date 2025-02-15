use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::future::Future;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use futures::future::try_join_all;
use log::info;
use serde::{Deserialize, Serialize};
use subst::VariableMap;

#[cfg(any(feature = "mbtiles", feature = "pmtiles", feature = "sprites"))]
use crate::file_config::FileConfigEnum;
#[cfg(feature = "fonts")]
use crate::fonts::FontSources;
use crate::source::{TileInfoSources, TileSources};
#[cfg(feature = "sprites")]
use crate::sprites::{SpriteConfig, SpriteSources};
use crate::srv::SrvConfig;
use crate::MartinError::{ConfigLoadError, ConfigParseError, ConfigWriteError, NoSources};
use crate::{IdResolver, MartinResult, OptOneMany};

pub type UnrecognizedValues = HashMap<String, serde_yaml::Value>;

pub struct ServerState {
    pub tiles: TileSources,
    #[cfg(feature = "sprites")]
    pub sprites: SpriteSources,
    #[cfg(feature = "fonts")]
    pub fonts: FontSources,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub srv: SrvConfig,

    #[cfg(feature = "postgres")]
    #[serde(default, skip_serializing_if = "OptOneMany::is_none")]
    pub postgres: OptOneMany<crate::pg::PgConfig>,

    #[cfg(feature = "pmtiles")]
    #[serde(default, skip_serializing_if = "FileConfigEnum::is_none")]
    pub pmtiles: FileConfigEnum<crate::pmtiles::PmtConfig>,

    #[cfg(feature = "mbtiles")]
    #[serde(default, skip_serializing_if = "FileConfigEnum::is_none")]
    pub mbtiles: FileConfigEnum<crate::mbtiles::MbtConfig>,

    #[cfg(feature = "sprites")]
    #[serde(default, skip_serializing_if = "FileConfigEnum::is_none")]
    pub sprites: FileConfigEnum<SpriteConfig>,

    #[serde(default, skip_serializing_if = "OptOneMany::is_none")]
    pub fonts: OptOneMany<PathBuf>,

    #[serde(flatten)]
    pub unrecognized: UnrecognizedValues,
}

impl Config {
    /// Apply defaults to the config, and validate if there is a connection string
    pub fn finalize(&mut self) -> MartinResult<UnrecognizedValues> {
        let mut res = UnrecognizedValues::new();
        copy_unrecognized_config(&mut res, "", &self.unrecognized);

        #[cfg(feature = "postgres")]
        for pg in self.postgres.iter_mut() {
            res.extend(pg.finalize()?);
        }

        #[cfg(feature = "pmtiles")]
        res.extend(self.pmtiles.finalize("pmtiles.")?);

        #[cfg(feature = "mbtiles")]
        res.extend(self.mbtiles.finalize("mbtiles.")?);

        #[cfg(feature = "sprites")]
        res.extend(self.sprites.finalize("sprites.")?);

        // TODO: support for unrecognized fonts?
        // res.extend(self.fonts.finalize("fonts.")?);

        let is_empty = true;

        #[cfg(feature = "postgres")]
        let is_empty = is_empty && self.postgres.is_empty();

        #[cfg(feature = "pmtiles")]
        let is_empty = is_empty && self.pmtiles.is_empty();

        #[cfg(feature = "mbtiles")]
        let is_empty = is_empty && self.mbtiles.is_empty();

        #[cfg(feature = "sprites")]
        let is_empty = is_empty && self.sprites.is_empty();

        #[cfg(feature = "fonts")]
        let is_empty = is_empty && self.fonts.is_empty();

        if is_empty {
            Err(NoSources)
        } else {
            Ok(res)
        }
    }

    pub async fn resolve(&mut self, idr: IdResolver) -> MartinResult<ServerState> {
        Ok(ServerState {
            tiles: self.resolve_tile_sources(idr).await?,
            #[cfg(feature = "sprites")]
            sprites: SpriteSources::resolve(&mut self.sprites)?,
            #[cfg(feature = "fonts")]
            fonts: FontSources::resolve(&mut self.fonts)?,
        })
    }

    async fn resolve_tile_sources(
        &mut self,
        #[allow(unused_variables)] idr: IdResolver,
    ) -> MartinResult<TileSources> {
        #[allow(unused_mut)]
        let mut sources: Vec<Pin<Box<dyn Future<Output = MartinResult<TileInfoSources>>>>> =
            Vec::new();

        #[cfg(feature = "postgres")]
        for s in self.postgres.iter_mut() {
            sources.push(Box::pin(s.resolve(idr.clone())));
        }

        #[cfg(feature = "pmtiles")]
        if !self.pmtiles.is_empty() {
            let cfg = &mut self.pmtiles;
            let val = crate::file_config::resolve_files(cfg, idr.clone(), "pmtiles");
            sources.push(Box::pin(val));
        }

        #[cfg(feature = "mbtiles")]
        if !self.mbtiles.is_empty() {
            let cfg = &mut self.mbtiles;
            let val = crate::file_config::resolve_files(cfg, idr.clone(), "mbtiles");
            sources.push(Box::pin(val));
        }

        Ok(TileSources::new(try_join_all(sources).await?))
    }

    pub fn save_to_file(&self, file_name: PathBuf) -> MartinResult<()> {
        let yaml = serde_yaml::to_string(&self).expect("Unable to serialize config");
        if file_name.as_os_str() == OsStr::new("-") {
            info!("Current system configuration:");
            println!("\n\n{yaml}\n");
            Ok(())
        } else {
            info!(
                "Saving config to {}, use --config to load it",
                file_name.display()
            );
            match File::create(&file_name) {
                Ok(mut file) => file
                    .write_all(yaml.as_bytes())
                    .map_err(|e| ConfigWriteError(e, file_name)),
                Err(e) => Err(ConfigWriteError(e, file_name)),
            }
        }
    }
}

pub fn copy_unrecognized_config(
    result: &mut UnrecognizedValues,
    prefix: &str,
    unrecognized: &UnrecognizedValues,
) {
    result.extend(
        unrecognized
            .iter()
            .map(|(k, v)| (format!("{prefix}{k}"), v.clone())),
    );
}

/// Read config from a file
pub fn read_config<'a, M>(file_name: &Path, env: &'a M) -> MartinResult<Config>
where
    M: VariableMap<'a>,
    M::Value: AsRef<str>,
{
    let mut file = File::open(file_name).map_err(|e| ConfigLoadError(e, file_name.into()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| ConfigLoadError(e, file_name.into()))?;
    parse_config(&contents, env, file_name)
}

pub fn parse_config<'a, M>(contents: &str, env: &'a M, file_name: &Path) -> MartinResult<Config>
where
    M: VariableMap<'a>,
    M::Value: AsRef<str>,
{
    subst::yaml::from_str(contents, env).map_err(|e| ConfigParseError(e, file_name.into()))
}

#[cfg(feature = "postgres")]
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::config::Config;
    use crate::test_utils::FauxEnv;

    pub fn parse_cfg(yaml: &str) -> Config {
        parse_config(yaml, &FauxEnv::default(), Path::new("<test>")).unwrap()
    }

    pub fn assert_config(yaml: &str, expected: &Config) {
        let mut config = parse_cfg(yaml);
        let res = config.finalize().unwrap();
        assert!(res.is_empty(), "unrecognized config: {res:?}");
        assert_eq!(&config, expected);
    }
}
