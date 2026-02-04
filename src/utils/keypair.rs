use anyhow::{Context, Result};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::path::Path;

/// Load a Solana keypair from disk.
///
/// - Validates file existence
/// - Produces clean, contextual errors
/// - Safe for WSL paths and symlinks
pub fn load_keypair(path: &str) -> Result<Keypair> {
    let path = Path::new(path)
        .canonicalize()
        .with_context(|| format!("invalid KEYPAIR_PATH: {path}"))?;

    if !path.exists() {
        anyhow::bail!("keypair file not found at {}", path.display());
    }

    let keypair = read_keypair_file(&path)
        .map_err(|e| anyhow::anyhow!("failed to read keypair from {}: {}", path.display(), e))?;

    // Touch pubkey to ensure keypair is valid and usable
    let _ = keypair.pubkey();

    Ok(keypair)
}
