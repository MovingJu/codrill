mod git;
mod manifest;
mod state;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "codrill",
    about = "실전처럼 코드를 읽고 대처하는 연습용 CLI",
    long_about = "실전처럼 코드를 읽고 대처하는 연습용 CLI.\n\n\
        장애 대응, 코드 리뷰, 보안 취약점 같은 상황을 git 저장소 하나(시나리오)로 받아서,\n\
        실제로 그 코드를 열어보고 조사하고 고쳐보는 도구입니다.\n\n\
        기본 흐름:\n  \
        1) codrill start <시나리오>   -- 받아서 상황 설명(BRIEF.md) 확인\n  \
        2) 평소 하던 대로 코드를 조사/수정\n  \
        3) 막히면 codrill hint, 다 풀었으면 codrill grade\n  \
        4) 답이 궁금하면 codrill reveal"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 시나리오 시작 -- 원격은 위치 인자로, 로컬은 반드시 --path로
    #[command(long_about = "\
시나리오를 받아서 현재 폴더에 풉니다. 둘 중 하나만 주세요(동시에 주면 에러):\n\n  \
  - git 원격 주소(위치 인자):  codrill start https://github.com/누군가/시나리오.git\n  \
  - 로컬 경로(--path):         codrill start --path ../내가-만든-시나리오\n\n\
(둘 다 내부적으로 `git clone <출처> .`를 쓰기 때문에, --path로 주는 로컬 경로도 실제 git\n\
저장소여야 합니다. 아직 커밋 안 한 변경사항은 안 딸려오니, 로컬 시나리오를 만드는 중이면\n\
커밋부터 하고 시작하세요.)\n\n\
기본은 지금 있는 폴더에 파일이 바로 풀립니다 -- 그래서 빈 폴더에서 실행하는 게 정상입니다.\n\
이미 뭔가 들어있는 폴더에서 시작하고 싶으면 -o로 하위 폴더 이름을 지정하세요.")]
    Start {
        /// git 원격 주소(https://... / git@...) -- 로컬 경로는 여기 말고 --path로
        source: Option<String>,
        /// 로컬 git 저장소 경로 -- 로컬로 시작할 땐 위치 인자 대신 반드시 이걸 쓰세요
        #[arg(long = "path", value_name = "경로")]
        path: Option<String>,
        /// 지금 폴더에 바로 풀지 않고, 이 이름의 하위 폴더를 새로 만들어 그 안에 클론
        #[arg(short = 'o', long = "into", value_name = "폴더명")]
        into: Option<String>,
    },
    /// 힌트 하나 공개 (다시 숨길 방법은 없음 -- 신중히)
    Hint,
    /// 정답 공개 (.codrill/SOLUTION.md 출력)
    Reveal,
    /// verify/run 실행해서 통과 여부 확인
    Grade,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("현재 디렉토리를 못 가져옴")?;

    match cli.command {
        Commands::Start { source, path, into } => cmd_start(&cwd, source, path, into.as_deref()),
        Commands::Hint => cmd_hint(&cwd),
        Commands::Reveal => cmd_reveal(&cwd),
        Commands::Grade => cmd_grade(&cwd),
    }
}

/// 이 cwd에서 진행 중인 시나리오의 힌트/정답을 숨겨두는 곳. state.toml도 여기 같이 둬서
/// ".codrill이 두 군데" 문제(예전엔 cwd랑 클론 폴더 안에 하나씩 따로 있었음)를 없앤다 --
/// 클론을 cwd에 바로 풀든(-o 없이) 하위 폴더에 풀든(-o) .codrill은 항상 cwd 딱 하나.
fn codrill_dir(cwd: &Path) -> PathBuf {
    cwd.join(".codrill")
}

fn cmd_start(
    cwd: &Path,
    source: Option<String>,
    path: Option<String>,
    into: Option<&str>,
) -> anyhow::Result<()> {
    // source(원격 URL 위치 인자)와 --path(로컬 경로)는 상호 배타적 -- 사용자가 계속 이 둘을
    // 헷갈려해서(로컬 경로를 위치 인자로 넣거나, 반대로 URL을 --path에 넣거나) 아예 인자
    // 단계에서 명확히 갈라놓는다.
    let source = match (source, path) {
        (Some(_), Some(_)) => anyhow::bail!(
            "URL과 --path를 동시에 줄 수 없습니다. 둘 중 하나만 쓰세요:\n  \
             codrill start <git-URL>       # 원격 저장소\n  \
             codrill start --path <경로>   # 로컬 저장소"
        ),
        (Some(s), None) => s,
        (None, Some(p)) => p,
        (None, None) => anyhow::bail!(
            "시나리오 출처를 지정하세요:\n  \
             codrill start <git-URL>       # 원격 저장소\n  \
             codrill start --path <경로>   # 로컬 저장소"
        ),
    };
    let source = source.as_str();

    // 이미 진행 중인 게 있으면 덮어쓰기 전에 알린다 -- 그냥 새로 클론해버리면
    // 이전 작업 디렉토리가 state에서 끊겨 고아가 된다(작업 내용 유실처럼 보임).
    if let Ok(prev) = state::load(cwd) {
        anyhow::bail!(
            "이미 '{}' 시나리오가 진행 중입니다 ({}).\n\
             계속하려면 그 폴더에서 작업하시고, 새로 시작하려면 .codrill/state.toml을 지우세요.",
            prev.active,
            prev.repo_path.display()
        );
    }

    // -o 없으면 지금 폴더에 그대로 풀고(git clone <source> .), -o가 있으면 그 이름의
    // 하위 폴더를 만들어 그 안에 푼다.
    let dest = match into {
        Some(name) => git::unique_dest(&cwd.join(name)),
        None => cwd.to_path_buf(),
    };

    // "빈 폴더가 아니라 클론이 안 됐다"는 힌트는 실제로 dest가 비어있지 않을 때만 붙인다.
    // dest가 비어있는데도 이 힌트를 무조건 붙이면(예전 버그), 진짜 원인(소스 경로가 틀렸다든지
    // 네트워크 문제든지)을 가려버리고 엉뚱한 데를 보게 만든다.
    let dest_nonempty = into.is_none()
        && std::fs::read_dir(&dest)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

    println!("클론 중: {source} -> {}", dest.display());
    git::clone(source, &dest).map_err(|e| {
        if dest_nonempty {
            e.context(
                "현재 폴더가 비어있지 않아서 여기 바로 풀 수 없습니다 -- \
                 빈 폴더에서 실행하거나 `-o <이름>`으로 하위 폴더에 클론하세요.",
            )
        } else {
            e
        }
    })?;

    let codrill_dir = codrill_dir(cwd);

    // 여기부터 실패하면 방금 클론한 내용이 쓰레기로 남으므로, 실패시 지우고 나간다.
    let prepared = (|| -> anyhow::Result<(manifest::Manifest, String)> {
        let manifest = manifest::load(&dest).context(
            "codrill.toml이 없거나 형식이 잘못됐습니다 -- 이 저장소가 codrill 시나리오가 맞는지 확인하세요",
        )?;
        let briefing_path = dest.join("BRIEF.md");
        let briefing = std::fs::read_to_string(&briefing_path)
            .with_context(|| format!("BRIEF.md이 없습니다: {}", briefing_path.display()))?;

        // .codrill/이 클론된 저장소 안(dest)에 놓이는 경우(=-o 없이 바로 푼 경우)에만
        // 로컬 전용 gitignore(.git/info/exclude)에 등록한다 -- 제작자의 레포/커밋은 안
        // 건드리고, 이 클론 하나에서만 무시되게 한다. -o로 바깥(cwd)에 뒀으면 애초에
        // 저장소 밖이라 git이 신경 쓸 필요가 없다.
        if codrill_dir.starts_with(&dest) {
            let exclude_path = dest.join(".git").join("info").join("exclude");
            if let Ok(mut existing) = std::fs::read_to_string(&exclude_path)
                && !existing.contains(".codrill/")
            {
                existing.push_str("\n.codrill/\n");
                std::fs::write(&exclude_path, existing)?;
            }
        }

        // 루트에 SOLUTION.md/HINTS.md가 평범하게 커밋돼있으면(제작자는 이게 정상),
        // 로컬 작업 폴더에서만 .codrill/ 안으로 옮겨서 조사하는 동안 눈에 안 띄게 한다.
        // 둘 다 같은 .codrill/ 안에 두니 "정답 옆에 힌트도" 요구사항이 자동으로 맞는다.
        std::fs::create_dir_all(&codrill_dir)?;
        for f in ["SOLUTION.md", "HINTS.md"] {
            let src = dest.join(f);
            if src.exists() {
                std::fs::rename(&src, codrill_dir.join(f))?;
            }
        }

        Ok((manifest, briefing))
    })();

    let (manifest, briefing) = match prepared {
        Ok(v) => v,
        Err(e) => {
            if into.is_some() {
                let _ = std::fs::remove_dir_all(&dest); // -o로 새로 만든 폴더니 통째로 정리
            }
            return Err(e);
        }
    };

    state::save(
        cwd,
        &state::State {
            active: manifest.scenario.name.clone(),
            repo_path: dest.clone(),
            hints_revealed: 0,
        },
    )?;

    println!();
    println!("=== {} ===", manifest.scenario.title);
    println!(
        "[{} / {}]",
        manifest.scenario.category, manifest.scenario.difficulty
    );
    if !manifest.scenario.tags.is_empty() {
        println!("tags: {}", manifest.scenario.tags.join(", "));
    }
    println!();
    println!("{briefing}");
    println!("---");
    if into.is_some() {
        println!(
            "여기서부터는 평소 하던 대로 조사하면 됩니다 (cd {}). 막히면 `codrill hint`.",
            dest.display()
        );
    } else {
        println!("여기서부터는 평소 하던 대로 조사하면 됩니다. 막히면 `codrill hint`.");
    }

    Ok(())
}

/// HINTS.md를 "---"로만 된 줄 기준으로 힌트 여러 개로 쪼갠다. 파일이 없으면 힌트 없음.
fn read_hints(codrill_dir: &Path) -> Vec<String> {
    let text = match std::fs::read_to_string(codrill_dir.join("HINTS.md")) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    text.split("\n---\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn cmd_hint(cwd: &Path) -> anyhow::Result<()> {
    let mut st = state::load(cwd)?;
    let hints = read_hints(&codrill_dir(cwd));

    if st.hints_revealed >= hints.len() {
        if hints.is_empty() {
            println!("이 시나리오엔 힌트가 없습니다.");
        } else {
            println!("힌트를 이미 다 봤습니다 ({}/{}).", st.hints_revealed, hints.len());
        }
        return Ok(());
    }

    let hint = &hints[st.hints_revealed];
    st.hints_revealed += 1;
    println!("힌트 {}/{}: {hint}", st.hints_revealed, hints.len());
    state::save(cwd, &st)?;
    Ok(())
}

fn cmd_reveal(cwd: &Path) -> anyhow::Result<()> {
    state::load(cwd)?; // 진행 중인 시나리오가 있는지만 확인

    let solution_path = codrill_dir(cwd).join("SOLUTION.md");
    let solution = std::fs::read_to_string(&solution_path).with_context(|| {
        format!(
            "SOLUTION.md이 없습니다: {} -- 이 시나리오엔 정답 해설이 없습니다",
            solution_path.display()
        )
    })?;

    println!("=== 정답 공개 ===");
    println!();
    println!("{solution}");
    Ok(())
}

/// verify/run이 빌드 산출물(target/, .venv/, __pycache__/ 등)을 만들어도 참가자의 실제
/// 작업 디렉토리(st.repo_path)를 더럽히지 않도록, .codrill/build/ 안에 작업 디렉토리를
/// 복사해두고 거기서 verify/run을 돌린다. grade를 다시 돌릴 때마다(재채점) 최신 코드가
/// 반영돼야 하므로 매번 다시 동기화하되, 통째로 지우고 새로 복사하면 build/ 안에 이미 쌓인
/// target/.venv 같은 빌드 캐시까지 날아가 매번 풀빌드가 되므로 rsync -a --delete로
/// "바뀐 파일만 갱신 + 지워진 파일 반영" 방식으로 동기화한다.
const SYNC_EXCLUDES: &[&str] = &[".git/", "target/", ".venv/", "venv/", "__pycache__/", ".codrill/"];

fn sync_build_dir(repo_path: &Path, build_dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(build_dir)
        .with_context(|| format!("{}을 못 만들었습니다", build_dir.display()))?;

    let rsync_available = Command::new("rsync").arg("--version").output().is_ok();

    if rsync_available {
        let mut src = repo_path.display().to_string();
        if !src.ends_with('/') {
            src.push('/'); // 끝에 /가 있어야 rsync가 폴더 자체가 아니라 내용물을 복사한다
        }

        let mut cmd = Command::new("rsync");
        cmd.arg("-a").arg("--delete");
        for ex in SYNC_EXCLUDES {
            cmd.arg(format!("--exclude={ex}"));
        }
        cmd.arg(&src).arg(build_dir);

        let status = cmd.status().context("rsync 실행 실패")?;
        if !status.success() {
            anyhow::bail!(
                "작업 디렉토리를 {}(으)로 동기화하는 데 실패했습니다 (rsync exit code {:?})",
                build_dir.display(),
                status.code()
            );
        }
    } else {
        // rsync가 없으면 증분 동기화(변경분만 갱신, 삭제 반영)는 못하지만, 최소한 최신
        // 코드는 반영되도록 순수 Rust 재귀 복사로 대체한다 -- 빌드 캐시 재사용은 못함.
        println!(
            "rsync를 찾을 수 없어 단순 복사로 대체합니다 \
             (증분 동기화가 아니라 매번 느릴 수 있고, 삭제된 파일이 안 지워질 수 있습니다)."
        );
        copy_dir_excluding(repo_path, build_dir, SYNC_EXCLUDES)
            .context("빌드 디렉토리 동기화 실패")?;
    }

    Ok(())
}

/// rsync 없을 때 쓰는 폴백: src 아래를 dst로 재귀 복사하되 exclude 목록에 있는 이름의
/// 디렉토리는 건너뛴다. 심볼릭 링크는 안전하게 그냥 건너뛴다.
fn copy_dir_excluding(src: &Path, dst: &Path, excludes: &[&str]) -> anyhow::Result<()> {
    let exclude_names: Vec<&str> = excludes.iter().map(|e| e.trim_end_matches('/')).collect();

    for entry in std::fs::read_dir(src).with_context(|| format!("{}을 못 읽음", src.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        if exclude_names.contains(&name.to_string_lossy().as_ref()) {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_dir_excluding(&src_path, &dst_path, excludes)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dst_path)
                .with_context(|| format!("{} 복사 실패", src_path.display()))?;
        }
        // 심볼릭 링크 등은 폴백 경로에서는 건너뛴다 -- rsync 쪽이 정상 경로.
    }
    Ok(())
}

fn cmd_grade(cwd: &Path) -> anyhow::Result<()> {
    let st = state::load(cwd)?;
    let build_dir = codrill_dir(cwd).join("build");

    println!("작업 디렉토리 동기화 중...");
    sync_build_dir(&st.repo_path, &build_dir)?;

    let verify_path = build_dir.join("verify").join("run");

    if !verify_path.exists() {
        anyhow::bail!(
            "verify/run이 없습니다: {} -- 이 시나리오엔 자동 채점이 없습니다",
            verify_path.display()
        );
    }

    println!("verify/run 실행 중...");
    let status = Command::new(&verify_path)
        .current_dir(&build_dir)
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                anyhow::anyhow!(
                    "verify/run에 실행 권한이 없습니다: {}\n\
                     시나리오 제작자라면 `chmod +x verify/run` 후 커밋하세요.",
                    verify_path.display()
                )
            } else {
                anyhow::anyhow!("verify/run 실행 실패 ({}): {e}", verify_path.display())
            }
        })?;

    if status.success() {
        println!("PASS — 통과했습니다.");
    } else {
        println!(
            "FAIL — 아직 통과 못했습니다 (exit code {}).",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
