//! In-app SSH key generation for Git remotes (§5.1). The private key goes to platform secure
//! storage via the app; only the public key is shown to the user.

use ssh_key::{Algorithm, LineEnding, PrivateKey, rand_core::OsRng};

use crate::{Error, Result};

pub struct SshKeyPair {
    /// OpenSSH PEM text (`-----BEGIN OPENSSH PRIVATE KEY-----`).
    pub private_openssh: String,
    /// `ssh-ed25519 AAAA… comment`
    pub public_openssh: String,
}

pub fn generate_ed25519(comment: &str) -> Result<SshKeyPair> {
    let mut key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)
        .map_err(|e| Error::Other(e.to_string()))?;
    key.set_comment(comment);
    let private_openssh = key
        .to_openssh(LineEnding::LF)
        .map_err(|e| Error::Other(e.to_string()))?
        .to_string();
    let public_openssh = key
        .public_key()
        .to_openssh()
        .map_err(|e| Error::Other(e.to_string()))?;
    Ok(SshKeyPair {
        private_openssh,
        public_openssh,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn generates_openssh_pair() {
        let k = super::generate_ed25519("daftar@pixel-8").unwrap();
        assert!(
            k.private_openssh
                .starts_with("-----BEGIN OPENSSH PRIVATE KEY-----")
        );
        assert!(
            k.public_openssh.starts_with("ssh-ed25519 ")
                && k.public_openssh.ends_with("daftar@pixel-8")
        );
    }
}
