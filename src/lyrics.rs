use anyhow::Result;
use once_cell::sync::OnceCell;
use reqwest::Client;

use crate::ffi::LyricLine;

#[derive(Debug, Clone)]
pub struct Lyrics(pub Vec<LyricLine>);
pub trait LyricSource {
    async fn fetch_lyrics(artist: &str, title: &str) -> Result<Vec<Lyrics>>;
}

pub struct LocalLyricSource;
impl LyricSource for LocalLyricSource {
    async fn fetch_lyrics(artist: &str, title: &str) -> Result<Vec<Lyrics>> {
        // TODO: cache lyrics in user db
        Ok(vec![])
    }
}

pub static CLIENT: OnceCell<Client> = OnceCell::new();

pub fn initialize() -> Result<()> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
        .build()?;
    CLIENT
        .set(client)
        .expect("Client should only be initialized once");
    Ok(())
}
