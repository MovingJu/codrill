use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Command;

/// `source`를 `dest` 폴더로 클론한다 (전체 브랜치 다 받음 -- 나중에 solution 브랜치도
/// 네트워크 왕복 없이 바로 체크아웃할 수 있게).
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

pub fn checkout(repo_dir: &Path, branch: &str) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["-C"])
        .arg(repo_dir)
        .args(["checkout", branch])
        .status()
        .context("git checkout 실행 실패")?;
    if !status.success() {
        bail!("'{branch}' 브랜치로 체크아웃하지 못했습니다 -- 이 시나리오 저장소에 해당 브랜치가 있는지 확인하세요");
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
