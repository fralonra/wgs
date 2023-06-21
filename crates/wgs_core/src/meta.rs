use crate::VERSION;
use binrw::{binrw, NullString};

/// The meta info of a wgs file.
#[derive(Debug)]
#[binrw]
#[brw(little)]
pub struct Meta {
    /// The name of the wgs file. Not filename.
    pub name: NullString,
    /// The count of textures embedded in the wgs file.
    pub texture_count: u8,
    /// The version of the wgs file.
    pub version: u32,
}

impl Meta {
    pub fn new(name: &str) -> Self {
        let name = NullString(name.as_bytes().to_vec());

        Self {
            name,
            texture_count: 0,
            version: VERSION,
        }
    }
}
