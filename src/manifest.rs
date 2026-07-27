use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct Manifest {
    pub scenario: ScenarioMeta,
}

#[derive(Deserialize, Debug)]
pub struct ScenarioMeta {
    pub name: String,
    pub title: String,
    pub category: String,
    pub difficulty: Difficulty,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        };
        write!(f, "{s}")
    }
}

pub fn load(repo_dir: &Path) -> anyhow::Result<Manifest> {
    let path = repo_dir.join("codrill.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("codrill.toml을 못 읽음 ({}): {e}", path.display()))?;
    let manifest: Manifest = toml::from_str(&text)
        .map_err(|e| anyhow::anyhow!("codrill.toml 형식이 이상함: {e}"))?;
    Ok(manifest)
}
