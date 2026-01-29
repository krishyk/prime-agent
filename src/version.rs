use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const VERSION_FILE: &str = "VERSION";

#[derive(Debug, Clone, Copy)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    pub fn load_bump_and_save() -> Result<Self> {
        let path = Path::new(VERSION_FILE);
        let current = if path.exists() {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("failed to read version file: {}", path.display()))?;
            Self::parse(&contents)?
        } else {
            Self::new(0, 1, 0)
        };
        let next = current.bump_patch();
        next.save(path)?;
        Ok(next)
    }

    #[must_use]
    pub fn as_string(self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow::anyhow!("invalid VERSION format"));
        }
        let major = parts[0].parse::<u64>().context("invalid major version")?;
        let minor = parts[1].parse::<u64>().context("invalid minor version")?;
        let patch = parts[2].parse::<u64>().context("invalid patch version")?;
        Ok(Self::new(major, minor, patch))
    }

    fn bump_patch(self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    fn save(self, path: &Path) -> Result<()> {
        let contents = format!("{}\n", self.as_string());
        fs::write(path, contents)
            .with_context(|| format!("failed to write version file: {}", path.display()))?;
        Ok(())
    }
}
