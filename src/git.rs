use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Command;

/// `source`를 `dest` 폴더로 클론한다.
pub fn clone(source: &str, dest: &Path) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["clone", source])
        .arg(dest)
        .status()
        .context("git clone 실행 실패 -- git이 설치돼있는지 확인하세요")?;
    if !status.success() {
        bail!("git clone이 실패했습니다: {source}");
    }
    Ok(())
}

/// dest가 이미 존재하면 그 안의 이름 뒤에 숫자를 붙여서 충돌을 피한다.
pub fn unique_dest(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }
    for i in 2.. {
        let candidate = base.with_file_name(format!(
            "{}-{i}",
            base.file_name().unwrap().to_string_lossy()
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}
