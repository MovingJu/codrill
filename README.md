# codrill

실전처럼 코드를 읽고 대처하는 연습용 CLI. 장애 대응, 코드 리뷰, 보안 취약점 같은 시나리오를
git 저장소 하나로 받아서, 실제로 조사하고 고쳐보는 도구.

## 사용법

```bash
# 빈 폴더에서 시작 -- 시나리오 파일이 지금 폴더에 바로 풀린다
mkdir my-attempt && cd my-attempt
codrill start https://github.com/누군가/시나리오.git   # 원격은 위치 인자로
# 로컬 경로로 시작할 땐 반드시 --path (위치 인자와 동시에 못 씀)
codrill start --path ../codrill-scenario-incident-1

# ... 평소 하던 대로 조사/수정 ...
codrill hint                            # 막히면 힌트 (HINTS.md, 하나씩 순서대로)
codrill grade                           # verify/run 실행해서 통과 여부 확인
codrill reveal                          # 정답 공개 (.codrill/SOLUTION.md 출력)
```

이미 파일이 있는 폴더에서 시작하고 싶으면(예: 여러 시나리오를 한 워크스페이스 안에 나란히
두고 싶을 때) `-o`로 하위 폴더 이름을 지정한다:

```bash
codrill start https://github.com/누군가/시나리오.git -o attempt-1
cd attempt-1   # 이 안에서 평소처럼 조사
```

`-o` 없이 쓸 때(기본값)와 다르게, 이 경우엔 `codrill hint`/`reveal`/`grade`를 방금 만든
하위 폴더가 아니라 **`start`를 실행했던 원래 위치**에서 실행해야 한다 -- 진행 상태(`.codrill/`)가
거기 저장되기 때문.

> 로컬 경로로 시작할 때 주의: 내부적으로 `git clone`을 쓰기 때문에, 대상 폴더가 실제 git
> 저장소여야 하고 **커밋 안 한 변경사항은 안 딸려온다**. 만들고 있는 시나리오를 테스트해보려면
> 먼저 커밋부터 할 것.

## 시나리오 저장소 만드는 법

시나리오 하나 = git 저장소 하나. 브랜치는 `main` 하나뿐 -- 제작자는 평범한 프로젝트처럼 만들면 된다:

```
<시나리오 레포>/
  codrill.toml   # 메타데이터
  BRIEF.md       # 상황 설명
  README.md      # 이 가짜 프로젝트의 구조 설명
  SOLUTION.md    # 정답 해설 (루트에 평범하게 커밋)
  HINTS.md       # 힌트, "---"로 줄만 딱 하나 있는 줄로 구분해서 순서대로 (루트에 평범하게 커밋)
  verify/run     # 채점 스크립트 (실행 권한 필수, exit 0 = 통과)
  (버그 있는 코드)
```

`codrill start`가 클론 직후에 `SOLUTION.md`와 `HINTS.md`를 로컬 작업 폴더의 `.codrill/`로
옮기고(`.git/info/exclude`에도 등록) 조사하는 동안 눈에 안 띄게 치워둔다 -- 제작자의 레포/커밋은
전혀 건드리지 않는, 순전히 로컬 전용 처리다. 그래서 이 저장소를 그냥 GitHub에서 훑어봐도
`codrill.toml`에 답이나 힌트가 평문으로 노출되지 않는다. `codrill reveal`/`hint`는 그 파일들을
다시 읽어서 보여줄 뿐, git 조작은 전혀 하지 않는다(커밋 안 한 변경사항이 있어도 안전).

`HINTS.md` 예시 (힌트 두 개):

```markdown
실패하는 요청과 실패 안 하는 요청의 차이를 먼저 봐라 -- 둘 다 커넥션을 똑같이 반납할까?
---
예외가 나는 경로에서 pool.release가 호출되는지 코드를 따라가봐라
```

`codrill.toml` 형식 (힌트는 여기 없음 -- 위 `HINTS.md` 참고):

```toml
[scenario]
name = "incident-1"
title = "..."
category = "incident-response"
difficulty = "medium"          # easy | medium | hard
tags = ["python", "database"]
```

예시 시나리오: [codrill-scenario-incident-1](https://github.com/MovingJu/codrill-scenario-incident-1)
(결제 API 커넥션 풀 고갈).

## 로드맵

- [x] `start` / `hint` / `reveal` / `grade` (MVP)
- [ ] 진행 기록(`.codrill/` 안에 커밋 로그 저장, attempt 브랜치 자동 생성)
- [ ] 시나리오 인덱스(레지스트리 — git 링크 목록만 들고 있는 가벼운 카탈로그)
- [ ] 레지스트리 밖 임의 verify/run 실행 시 경고(신뢰 등급 구분) — 공급망 문제, 레지스트리 열 때 반드시 같이 처리
