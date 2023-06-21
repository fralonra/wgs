use crate::{meta::Meta, texture::Texture, FRAG_DEFAULT};
use binrw::{binrw, BinRead, BinResult, BinWrite, NullString};
use std::io;

/// The core implementation of a wgs file.
#[derive(Debug)]
#[binrw]
#[brw(little)]
pub struct WgsData {
    meta: Meta,
    frag: NullString,
    #[br(count = meta.texture_count)]
    textures: Vec<Texture>,
}

impl Default for WgsData {
    fn default() -> Self {
        Self::new("Untitled", FRAG_DEFAULT)
    }
}

impl WgsData {
    /// Loads wgs data from a reader. [Read more](binrw::BinRead::read).
    pub fn load(reader: &mut (impl io::Read + io::Seek)) -> BinResult<Self> {
        Self::read(reader)
    }

    pub fn new(name: &str, frag: &str) -> Self {
        let meta = Meta::new(name);
        let frag = NullString(frag.as_bytes().to_vec());
        Self {
            meta,
            frag,
            textures: vec![],
        }
    }

    /// Adds a texture.
    pub fn add_texture(&mut self, width: u32, height: u32, data: Vec<u8>) {
        self.textures.push(Texture::new(width, height, data));
        self.meta.texture_count = self.textures.len() as u8;
    }

    /// Changes the texture of the current index.
    pub fn change_texture(&mut self, index: usize, width: u32, height: u32, data: Vec<u8>) {
        self.textures[index] = Texture::new(width, height, data);
        self.meta.texture_count = self.textures.len() as u8;
    }

    /// Returns the content of the editable part of the fragment shader.
    pub fn frag(&self) -> String {
        self.frag.to_string()
    }

    /// Returns the name of the wgs data. Not filename.
    pub fn name(&self) -> String {
        self.meta.name.to_string()
    }

    /// Removes a texture.
    pub fn remove_texture(&mut self, index: usize) {
        self.textures.remove(index);
        self.meta.texture_count = self.textures.len() as u8;
    }

    /// Save wgs data to the writer. [Read more](binrw::BinWrite::write).
    pub fn save(&self, writer: &mut (impl io::Write + io::Seek)) -> BinResult<()> {
        self.write(writer)
    }

    /// Sets the content of the editable part of the fragment shader.
    pub fn set_frag(&mut self, frag: &str) {
        self.frag.0 = frag.as_bytes().to_vec();
    }

    /// Sets the name for the wgs data.
    pub fn set_name(&mut self, name: &str) {
        self.meta.name.0 = name.as_bytes().to_vec();
    }

    /// Returns the textures embedded in the wgs data.
    pub fn textures_ref(&self) -> &Vec<Texture> {
        &self.textures
    }
}
