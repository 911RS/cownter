use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub const SERVICE: &str = "app.cownter.desktop";

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SecretError {
    #[error("the system keyring is not available")]
    Unavailable,
    #[error("keyring error: {0}")]
    Backend(String),
}

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, name: &str, value: &str) -> Result<(), SecretError>;
    fn delete(&self, name: &str) -> Result<(), SecretError>;
    fn available(&self) -> bool;
}

#[derive(Default)]
pub struct KeyringSecrets {
    probe: OnceLock<bool>,
}

impl KeyringSecrets {
    pub fn new() -> Self {
        Self::default()
    }

    fn entry(name: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(SERVICE, name).map_err(|e| SecretError::Backend(e.to_string()))
    }

    fn check(&self) -> Result<(), SecretError> {
        if self.available() { Ok(()) } else { Err(SecretError::Unavailable) }
    }
}

impl SecretStore for KeyringSecrets {
    fn get(&self, name: &str) -> Result<Option<String>, SecretError> {
        self.check()?;
        match Self::entry(name)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError::Backend(e.to_string())),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), SecretError> {
        self.check()?;
        Self::entry(name)?.set_password(value).map_err(|e| SecretError::Backend(e.to_string()))
    }

    fn delete(&self, name: &str) -> Result<(), SecretError> {
        self.check()?;
        match Self::entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretError::Backend(e.to_string())),
        }
    }

    fn available(&self) -> bool {
        *self.probe.get_or_init(|| {
            let Ok(e) = keyring::Entry::new(SERVICE, "__probe") else { return false };
            if e.set_password("probe").is_err() {
                return false;
            }
            let ok = e.get_password().is_ok();
            let _ = e.delete_credential();
            ok
        })
    }
}

#[derive(Default)]
pub struct MemorySecrets {
    map: Mutex<HashMap<String, String>>,
}

impl MemorySecrets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(pairs: &[(&str, &str)]) -> Self {
        let s = Self::default();
        for (k, v) in pairs {
            s.map.lock().unwrap().insert(k.to_string(), v.to_string());
        }
        s
    }
}

impl SecretStore for MemorySecrets {
    fn get(&self, name: &str) -> Result<Option<String>, SecretError> {
        Ok(self.map.lock().unwrap().get(name).cloned())
    }
    fn set(&self, name: &str, value: &str) -> Result<(), SecretError> {
        self.map.lock().unwrap().insert(name.into(), value.into());
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<(), SecretError> {
        self.map.lock().unwrap().remove(name);
        Ok(())
    }
    fn available(&self) -> bool {
        true
    }
}

/// Reads from an in-memory overlay first, then the base store. Writes go to the overlay only.
pub struct LayeredSecrets {
    pub top: MemorySecrets,
    pub base: Arc<dyn SecretStore>,
}

impl SecretStore for LayeredSecrets {
    fn get(&self, name: &str) -> Result<Option<String>, SecretError> {
        match self.top.get(name)? {
            Some(v) => Ok(Some(v)),
            None => self.base.get(name),
        }
    }
    fn set(&self, name: &str, value: &str) -> Result<(), SecretError> {
        self.top.set(name, value)
    }
    fn delete(&self, name: &str) -> Result<(), SecretError> {
        self.top.delete(name)
    }
    fn available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_round_trip() {
        let m = MemorySecrets::new();
        assert_eq!(m.get("x").unwrap(), None);
        m.set("x", "1").unwrap();
        assert_eq!(m.get("x").unwrap().as_deref(), Some("1"));
        m.delete("x").unwrap();
        assert_eq!(m.get("x").unwrap(), None);
    }

    #[test]
    fn layered_prefers_overlay() {
        let base: Arc<dyn SecretStore> = Arc::new(MemorySecrets::with(&[("a", "base"), ("b", "base")]));
        let l = LayeredSecrets { top: MemorySecrets::with(&[("a", "top")]), base };
        assert_eq!(l.get("a").unwrap().as_deref(), Some("top"));
        assert_eq!(l.get("b").unwrap().as_deref(), Some("base"));
    }

    #[test]
    #[ignore]
    fn keyring_round_trip() {
        let k = KeyringSecrets::new();
        assert!(k.available());
        k.set("__test", "v").unwrap();
        assert_eq!(k.get("__test").unwrap().as_deref(), Some("v"));
        k.delete("__test").unwrap();
        assert_eq!(k.get("__test").unwrap(), None);
    }
}
