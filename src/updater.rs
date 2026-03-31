use serde::Deserialize;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config::SharedState;

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

fn check_for_updates(state: &Arc<SharedState>) {
    let client = match reqwest::blocking::Client::builder()
        .user_agent("sunrise-brightness")
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let url = "https://api.github.com/repos/Vianpyro/sunrise-brightness/releases/latest";

    if let Ok(resp) = client.get(url).send()
        && let Ok(release) = resp.json::<GitHubRelease>()
    {
        let latest = release.tag_name.trim_start_matches('v').to_string();
        *state.latest_version.write().unwrap() = Some(latest);
    }
}

pub fn spawn(state: Arc<SharedState>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(5));
        check_for_updates(&state);
        loop {
            thread::sleep(Duration::from_secs(4 * 3600));
            check_for_updates(&state);
        }
    });
}
