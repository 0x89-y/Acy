/// Read a response body, giving up (None) once it passes `max` bytes, so a
/// misbehaving or hostile server can't make us buffer an unbounded download.
pub async fn read_capped(mut resp: reqwest::Response, max: usize) -> Option<Vec<u8>> {
    if resp.content_length().is_some_and(|n| n > max as u64) {
        return None;
    }
    let mut out = Vec::new();
    while let Some(chunk) = resp.chunk().await.ok()? {
        if out.len() + chunk.len() > max {
            return None;
        }
        out.extend_from_slice(&chunk);
    }
    Some(out)
}
