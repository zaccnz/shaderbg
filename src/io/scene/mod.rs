/*
 * This module handles the scene parsing from TOML
 */

use serde::Deserialize;

pub mod pass;
pub mod resource;
pub mod setting;
pub mod util;
use pass::*;
use resource::*;
use setting::*;
use util::*;

#[derive(Debug, Deserialize)]
pub struct Descriptor {
    pub meta: Metadata,
    pub settings: DeserializableMap<Setting>,
    pub resources: DeserializableMap<Resource>,
    pub render_passes: Vec<RenderPass>,
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}
