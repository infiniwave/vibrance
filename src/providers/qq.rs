use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use lrc::Lyrics;
use reqwest::Url;

use crate::{ffi::LyricLine, lyrics::{LyricSource, CLIENT}};

pub struct QQProvider;
impl LyricSource for QQProvider {
    async fn fetch_lyrics(artist: &str, title: &str) -> Result<Vec<crate::lyrics::Lyrics>> {
        let mut url = Url::parse("https://c.y.qq.com/splcloud/fcgi-bin/smartbox_new.fcg").expect("This URL should parse");
        url.query_pairs_mut()
            .append_pair("inCharset", "utf-8")
            .append_pair("outCharset", "utf-8")
            .append_pair("key", &format!("{title} {artist}"));
        let client = CLIENT.get().unwrap();
        let request = client.get(url).header("Referer", "http://y.qq.com/portal/player.html").send().await?;
        if !request.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch lyrics from QQ: HTTP {}", request.status()));
        }
        let response_text = request.text().await?;
        let json = serde_json::from_str::<serde_json::Value>(&response_text)?;
        let data = json.get("data")
            .and_then(|d| d.get("song"))
            .and_then(|s| s.get("itemlist"))
            .ok_or(anyhow::anyhow!("Invalid response format from QQ"))?
            .as_array()
            .ok_or(anyhow::anyhow!("Expected 'itemlist' to be an array"))?
            .to_owned();
        let mut lyrics = Vec::new();
        for item in data {
            let mid = item.get("mid")
                .and_then(|l| l.as_str())
                .ok_or(anyhow::anyhow!("Missing 'mid' in item"))?;
            let mut url = Url::parse("https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg").expect("This URL should parse");
            url.query_pairs_mut()
                .append_pair("songmid", mid)
                .append_pair("g_tk", "5381")
                .append_pair("format", "json")
                .append_pair("inCharset", "utf-8")
                .append_pair("outCharset", "utf-8");
            let request = client.get(url).header("Referer", "http://y.qq.com/portal/player.html").send().await?;
            if !request.status().is_success() {
                return Err(anyhow::anyhow!("Failed to fetch lyrics from QQ: HTTP {}", request.status()));
            }
            let response_text = request.text().await?;
            let json = serde_json::from_str::<serde_json::Value>(&response_text)?;
            let lyric = json.get("lyric")
                .and_then(|l| l.as_str())
                .ok_or(anyhow::anyhow!("Missing 'lyric' in response"))?
                .to_string();
            let result = String::from_utf8(BASE64_STANDARD.decode(lyric.as_bytes())?)?;
            let result = result.replace("&apos;", "'")
                .replace("&quot;", "\"")
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&nbsp;", " ")
                .replace("&ensp;", " ")
                .replace("&emsp;", " ");
            let parsed = Lyrics::from_str(result)?.get_timed_lines().iter().map(|line| {
                LyricLine {
                    timestamp: line.0.get_timestamp() as f64,
                    text: line.1.to_string(),
                }
            }).collect::<Vec<_>>();
            lyrics.push(crate::lyrics::Lyrics(parsed));
        }
        println!("Fetched lyrics: {:?}", lyrics);
        Ok(lyrics)

    }
}