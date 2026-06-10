use anyhow::anyhow;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

pub fn hash_password(plain: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| anyhow!("failed to hash password: {e}"))?
        .to_string();

    Ok(hash)
}

pub fn verify_password(plain: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow!("invalid stored password hash: {e}"))?;
    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(plain.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    #[test]
    fn hashes_and_verifies_password() {
        let hash = hash_password("Admin@123").expect("hash should be generated");
        let ok = verify_password("Admin@123", &hash).expect("verification should run");
        assert!(ok);
    }

    #[test]
    fn rejects_wrong_password() {
        let hash = hash_password("Admin@123").expect("hash should be generated");
        let ok = verify_password("Wrong@123", &hash).expect("verification should run");
        assert!(!ok);
    }
}
