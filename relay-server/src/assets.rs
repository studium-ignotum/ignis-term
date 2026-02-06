use rust_embed::RustEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "assets"]
pub struct Assets;
