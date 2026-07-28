mod git;
mod manifest;
mod state;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "codrill", about = "실전처럼 코드를 읽고 대처하는 연습용 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 시나리오 시작 (git 주소 또는 로컬 경로)
    Start { source: String },
    /// 힌트 하나 공개
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
        Commands::Start { source } => cmd_start(&cwd, &source),
        Commands::Hint => cmd_hint(&cwd),
        Commands::Reveal => cmd_reveal(&cwd),
        Commands::Grade => cmd_grade(&cwd),
    }
}

fn cmd_start(cwd: &Path, source: &str) -> anyhow::Result<()> {
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

    // 폴더 이름은 일단 source의 마지막 조각으로 추정, codrill.toml 읽은 뒤 name으로 검증만.
    let guessed_name = source
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("scenario")
        .trim_end_matches(".git");
    let dest = git::unique_dest(&cwd.join(guessed_name));

    println!("클론 중: {source} -> {}", dest.display());
    git::clone(source, &dest)?;

    // 여기부터 실패하면 방금 클론한 폴더는 쓰레기로 남으므로, 실패시 지우고 나간다.
    let prepared = (|| -> anyhow::Result<(manifest::Manifest, String)> {
        let manifest = manifest::load(&dest).context(
            "codrill.toml이 없거나 형식이 잘못됐습니다 -- 이 저장소가 codrill 시나리오가 맞는지 확인하세요",
        )?;
        let briefing_path = dest.join("BRIEF.md");
        let briefing = std::fs::read_to_string(&briefing_path)
            .with_context(|| format!("BRIEF.md이 없습니다: {}", briefing_path.display()))?;

        // .codrill/를 로컬 전용 gitignore(.git/info/exclude)에 등록 -- 제작자의 레포/커밋은
        // 안 건드리고, 이 클론 하나에서만 무시되게 한다.
        let exclude_path = dest.join(".git").join("info").join("exclude");
        if let Ok(mut existing) = std::fs::read_to_string(&exclude_path) {
            if !existing.contains(".codrill/") {
                existing.push_str("\n.codrill/\n");
                std::fs::write(&exclude_path, existing)?;
            }
        }

        // 루트에 SOLUTION.md가 평범하게 커밋돼있으면(제작자는 이게 정상), 로컬 작업 폴더에서만
        // .codrill/ 안으로 옮겨서 조사하는 동안 눈에 안 띄게 한다.
        let root_solution = dest.join("SOLUTION.md");
        if root_solution.exists() {
            let codrill_dir = dest.join(".codrill");
            std::fs::create_dir_all(&codrill_dir)?;
            std::fs::rename(&root_solution, codrill_dir.join("SOLUTION.md"))?;
        }

        Ok((manifest, briefing))
    })();

    let (manifest, briefing) = match prepared {
        Ok(v) => v,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dest); // 실패했으니 클론 흔적 정리
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
    println!(
        "여기서부터는 평소 하던 대로 조사하면 됩니다 (cd {}). 막히면 `codrill hint`.",
        dest.display()
    );

    Ok(())
}

fn cmd_hint(cwd: &Path) -> anyhow::Result<()> {
    let mut st = state::load(cwd)?;
    let manifest = manifest::load(&st.repo_path)?;

    if st.hints_revealed >= manifest.scenario.hints.len() {
        if manifest.scenario.hints.is_empty() {
            println!("이 시나리오엔 힌트가 없습니다.");
        } else {
            println!("힌트를 이미 다 봤습니다 ({}/{}).", st.hints_revealed, manifest.scenario.hints.len());
        }
        return Ok(());
    }

    let hint = &manifest.scenario.hints[st.hints_revealed];
    st.hints_revealed += 1;
    println!("힌트 {}/{}: {hint}", st.hints_revealed, manifest.scenario.hints.len());
    state::save(cwd, &st)?;
    Ok(())
}

fn cmd_reveal(cwd: &Path) -> anyhow::Result<()> {
    let st = state::load(cwd)?;

    let solution_path = st.repo_path.join(".codrill").join("SOLUTION.md");
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
