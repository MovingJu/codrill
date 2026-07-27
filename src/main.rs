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
    /// 정답 공개 (solution 브랜치로 체크아웃)
    Reveal,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("현재 디렉토리를 못 가져옴")?;

    match cli.command {
        Commands::Start { source } => cmd_start(&cwd, &source),
        Commands::Hint => cmd_hint(&cwd),
        Commands::Reveal => cmd_reveal(&cwd),
    }
}

fn cmd_start(cwd: &Path, source: &str) -> anyhow::Result<()> {
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
    git::checkout(&dest, "start")?;

    let manifest = manifest::load(&dest)
        .context("codrill.toml이 없거나 형식이 잘못됐습니다 -- 이 저장소가 codrill 시나리오가 맞는지 확인하세요")?;

    let briefing_path = dest.join("BRIEFING.md");
    let briefing = std::fs::read_to_string(&briefing_path)
        .with_context(|| format!("BRIEFING.md이 없습니다: {}", briefing_path.display()))?;

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
    git::checkout(&st.repo_path, "solution")?;

    let solution_path = st.repo_path.join("SOLUTION.md");
    let solution = std::fs::read_to_string(&solution_path).with_context(|| {
        format!(
            "SOLUTION.md이 없습니다: {} -- solution 브랜치에 이 파일이 있는지 확인하세요",
            solution_path.display()
        )
    })?;

    println!("=== 정답 공개 ===");
    println!();
    println!("{solution}");
    Ok(())
}
