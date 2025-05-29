use anyhow::Result;
use reqwest::Url;

use crate::lyrics::CLIENT;

pub async fn fetch_album_art(artist: &str, track: &str) -> Result<String> {
    let mut url = Url::parse("https://musicbrainz.org/ws/2/recording/").expect("Should have parsed");
    url.query_pairs_mut()
        .append_pair("query", &format!("artist:\"{}\" recording:\"{}\"", artist, track))
        .append_pair("fmt", "json");

    let client = CLIENT.get().expect("Client should be initialized");
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to fetch album art: {}", response.status()));
    }
    let json: serde_json::Value = response.json().await?;
    let data = json.get("recordings")
        .and_then(|r| r.as_array())
        .ok_or(anyhow::anyhow!("Invalid response format from MusicBrainz"))?
        .to_owned();
    println!("{:?}", data);
    for item in data {
        let cover_art = item.get("releases")
            .and_then(|r| r.as_array())
            .ok_or(anyhow::anyhow!("Missing 'releases' in item"))?
            .iter()
            .filter_map(|r| r.get("id"))
            .filter_map(|id| id.as_str())
            .collect::<Vec<_>>();
        for release in cover_art {
            if let Some(image) = fetch_release_image(release).await? {
                return Ok(image);
            }
        }
    }
    Err(anyhow::anyhow!("No album art found for {} - {}", artist, track))
}

pub async fn fetch_release_image(release: &str) -> Result<Option<String>> {
    let mut url = Url::parse("https://coverartarchive.org/release/").expect("Should have parsed");
    url.path_segments_mut().expect("Should have path segments").push(release);

    let client = CLIENT.get().expect("Client should be initialized");
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let json: serde_json::Value = response.json().await?;
    println!("{:?}", json);
    let images = json.get("images")
        .and_then(|i| i.as_array())
        .ok_or(anyhow::anyhow!("Invalid response format from Cover Art Archive"))?;
    
    if let Some(image) = images.first() {
        if let Some(image_url) = image.get("image") {
            return Ok(Some(image_url.as_str().ok_or(anyhow::anyhow!("Expected 'image' to be a string"))?.to_string()));
        }
    }
    Ok(None)
}