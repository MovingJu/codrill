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

/// 워킹 트리에 커밋 안 된 변경사항이 있으면 stash해서 치워둔다 (반환값: 실제로 stash했는지).
/// 아직 attempt 브랜치/커밋 기록 기능이 없어서, 유저가 커밋 없이 파일만 고친 상태로
/// reveal하는 게 지금은 제일 흔한 경로다 -- 그 변경사항을 지우지 않고 보존하는 게 목적.
pub fn stash_if_dirty(repo_dir: &Path) -> anyhow::Result<bool> {
    let status_out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["status", "--porcelain"])
        .output()
        .context("git status 실행 실패")?;
    if status_out.stdout.is_empty() {
        return Ok(false);
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["stash", "push", "-u", "-m", "codrill: auto-stash before reveal"])
        .status()
        .context("git stash 실행 실패")?;
    if !status.success() {
        bail!("커밋 안 된 변경사항을 stash하지 못했습니다");
    }
    Ok(true)
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
