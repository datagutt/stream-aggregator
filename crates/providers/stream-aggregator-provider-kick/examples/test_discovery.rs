use wreq::Client;
use wreq_util::Emulation;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().emulation(Emulation::Chrome131).build()?;

    // Test 1: Tag filter
    println!("=== Test 1: Tag filter (gaming) ===");
    let url = "https://web.kick.com/api/v1/livestreams?limit=3&sort=viewer_count_desc&language=en&tag=gaming";
    println!("URL: {}", url);
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(livestreams) = json["data"]["livestreams"].as_array() {
            println!("Found {} streams", livestreams.len());
            for stream in livestreams {
                println!(
                    "  - {} ({}): {} viewers - Tags: {:?}",
                    stream["channel"]["username"].as_str().unwrap_or("Unknown"),
                    stream["category"]["name"].as_str().unwrap_or("Unknown"),
                    stream["viewer_count"].as_u64().unwrap_or(0),
                    stream["tags"]
                );
            }
        }
    }

    println!("\n=== Test 2: Category ID filter (15 = Just Chatting) ===");
    let url2 =
        "https://web.kick.com/api/v1/livestreams?limit=3&sort=viewer_count_desc&category_id=15";
    println!("URL: {}", url2);
    let response2 = client.get(url2).send().await?;
    let text2 = response2.text().await?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text2) {
        if let Some(livestreams) = json["data"]["livestreams"].as_array() {
            println!("Found {} streams", livestreams.len());
            for stream in livestreams {
                println!(
                    "  - {} ({}): {} viewers",
                    stream["channel"]["username"].as_str().unwrap_or("Unknown"),
                    stream["category"]["name"].as_str().unwrap_or("Unknown"),
                    stream["viewer_count"].as_u64().unwrap_or(0)
                );
            }
        }
    }

    println!("\n=== Test 3: Multiple languages ===");
    let url3 = "https://web.kick.com/api/v1/livestreams?limit=3&sort=viewer_count_desc&language=sq&language=en";
    println!("URL: {}", url3);
    let response3 = client.get(url3).send().await?;
    let text3 = response3.text().await?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text3) {
        if let Some(livestreams) = json["data"]["livestreams"].as_array() {
            println!("Found {} streams", livestreams.len());
            for stream in livestreams {
                println!(
                    "  - {} ({}): Language: {}",
                    stream["channel"]["username"].as_str().unwrap_or("Unknown"),
                    stream["language"].as_str().unwrap_or("Unknown"),
                    stream["viewer_count"].as_u64().unwrap_or(0)
                );
            }
        }
    }

    Ok(())
}
