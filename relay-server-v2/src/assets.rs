use rust_embed::RustEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "web-ui/dist"]
pub struct Assets;
