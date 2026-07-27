use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const STATE_DIR: &str = ".codrill";
const STATE_FILE: &str = "state.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub active: String,
    pub repo_path: PathBuf,
    #[serde(default)]
    pub hints_revealed: usize,
}

fn state_path(cwd: &Path) -> PathBuf {
    cwd.join(STATE_DIR).join(STATE_FILE)
}

pub fn save(cwd: &Path, state: &State) -> anyhow::Result<()> {
    let dir = cwd.join(STATE_DIR);
    std::fs::create_dir_all(&dir)?;
    let text = toml::to_string_pretty(state)?;
    std::fs::write(state_path(cwd), text)?;
    Ok(())
}

pub fn load(cwd: &Path) -> anyhow::Result<State> {
    let path = state_path(cwd);
    let text = std::fs::read_to_string(&path).map_err(|_| {
        anyhow::anyhow!("진행 중인 시나리오가 없습니다 (`codrill start`로 먼저 시작하세요)")
    })?;
    let state: State = toml::from_str(&text)?;
    Ok(state)
}
