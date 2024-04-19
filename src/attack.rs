use crate::client::Client;

pub async fn open_to_targets(
    client: &mut Client,
    targets: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    for target in targets.iter() {
        let target = PublicKey::from_str(target)?;
        let _ = client.open_channel(target, 1_000_000, 1_000_000).await?;
    }
    Ok(())
}
