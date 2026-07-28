# codrill

실전처럼 코드를 읽고 대처하는 연습용 CLI. 장애 대응, 코드 리뷰, 보안 취약점 같은 시나리오를
git 저장소 하나로 받아서, 실제로 조사하고 고쳐보는 도구.

## 사용법

```bash
codrill start <git주소 또는 로컬경로>   # 시나리오 시작, BRIEF.md 출력
# ... 평소 하던 대로 조사 ...
codrill hint                            # 막히면 힌트
codrill grade                           # verify/run 실행해서 통과 여부 확인
codrill reveal                          # 정답 공개 (.codrill/SOLUTION.md 출력)
```

## 시나리오 저장소 만드는 법

시나리오 하나 = git 저장소 하나. 브랜치는 `main` 하나뿐 -- 제작자는 평범한 프로젝트처럼 만들면 된다:

```
<시나리오 레포>/
  codrill.toml   # 메타데이터
  BRIEF.md       # 상황 설명
  README.md      # 이 가짜 프로젝트의 구조 설명
  SOLUTION.md    # 정답 해설 (루트에 평범하게 커밋)
  verify/run     # 채점 스크립트 (실행 권한 필수, exit 0 = 통과)
  (버그 있는 코드)
```

`codrill start`가 클론 직후에 `SOLUTION.md`를 로컬 작업 폴더의 `.codrill/`로 옮기고
(`.git/info/exclude`에도 등록) 조사하는 동안 눈에 안 띄게 치워둔다 -- 제작자의 레포/커밋은
전혀 건드리지 않는, 순전히 로컬 전용 처리다. `codrill reveal`은 그 파일을 다시 읽어서 보여줄 뿐,
git 조작은 전혀 하지 않는다(커밋 안 한 변경사항이 있어도 안전).

`codrill.toml` 형식:

```toml
[scenario]
name = "incident-1"
title = "..."
category = "incident-response"
difficulty = "medium"          # easy | medium | hard
hints = ["...", "..."]
tags = ["python", "database"]
```

예시 시나리오: [codrill-scenario-incident-1](https://github.com/MovingJu/codrill-scenario-incident-1)
(결제 API 커넥션 풀 고갈).

## 로드맵

- [x] `start` / `hint` / `reveal` / `grade` (MVP)
- [ ] 진행 기록(`.codrill/` 안에 커밋 로그 저장, attempt 브랜치 자동 생성)
- [ ] 시나리오 인덱스(레지스트리 — git 링크 목록만 들고 있는 가벼운 카탈로그)
- [ ] 레지스트리 밖 임의 verify/run 실행 시 경고(신뢰 등급 구분) — 공급망 문제, 레지스트리 열 때 반드시 같이 처리
