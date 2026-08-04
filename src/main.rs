mod git;
mod manifest;
mod state;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

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
    /// 시나리오 시작 -- git 주소든 로컬 경로든 그대로 넣으면 됩니다
    #[command(long_about = "\
시나리오를 받아서 현재 폴더에 풉니다. SOURCE 자리엔 아래 둘 다 됩니다:\n\n  \
  - git 원격 주소:  codrill start https://github.com/누군가/시나리오.git\n  \
  - 로컬 경로:      codrill start ../내가-만든-시나리오\n\n\
(내부적으로 `git clone <SOURCE> .`를 쓰기 때문에, 로컬 경로여도 실제 git 저장소여야 합니다.\n\
아직 커밋 안 한 변경사항은 안 딸려오니, 로컬 시나리오를 만드는 중이면 커밋부터 하고 시작하세요.)\n\n\
기본은 지금 있는 폴더에 파일이 바로 풀립니다 -- 그래서 빈 폴더에서 실행하는 게 정상입니다.\n\
이미 뭔가 들어있는 폴더에서 시작하고 싶으면 -o로 하위 폴더 이름을 지정하세요.")]
    Start {
        /// git 원격 주소(https://... / git@...) 또는 로컬 git 저장소 경로
        source: String,
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
        Commands::Start { source, into } => cmd_start(&cwd, &source, into.as_deref()),
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

fn cmd_start(cwd: &Path, source: &str, into: Option<&str>) -> anyhow::Result<()> {
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

    println!("클론 중: {source} -> {}", dest.display());
    git::clone(source, &dest).map_err(|e| {
        if into.is_none() {
            e.context(
                "현재 폴더가 비어있지 않으면 여기 바로 풀 수 없습니다 -- \
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
            if let Ok(mut existing) = std::fs::read_to_string(&exclude_path) {
                if !existing.contains(".codrill/") {
                    existing.push_str("\n.codrill/\n");
                    std::fs::write(&exclude_path, existing)?;
                }
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

fn cmd_grade(cwd: &Path) -> anyhow::Result<()> {
    let st = state::load(cwd)?;
    let verify_path = st.repo_path.join("verify").join("run");

    if !verify_path.exists() {
        anyhow::bail!(
            "verify/run이 없습니다: {} -- 이 시나리오엔 자동 채점이 없습니다",
            verify_path.display()
        );
    }

    println!("verify/run 실행 중...");
    let status = std::process::Command::new(&verify_path)
        .current_dir(&st.repo_path)
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
