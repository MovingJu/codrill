# codrill

실전처럼 코드를 읽고 대처하는 연습용 CLI. 장애 대응, 코드 리뷰, 보안 취약점 같은 시나리오를
git 저장소 하나로 받아서, 실제로 조사하고 고쳐보는 도구.

## 사용법

```bash
codrill start <git주소 또는 로컬경로>   # 시나리오 시작, BRIEF.md 출력
# ... 평소 하던 대로 조사 ...
codrill hint                            # 막히면 힌트
codrill grade                           # verify/run 실행해서 통과 여부 확인
codrill reveal                          # 정답 공개 (solution 브랜치로 전환)
```

## 시나리오 저장소 만드는 법

시나리오 하나 = git 저장소 하나. `main`(기본 브랜치, git 표준 이름) / `solution` 두 브랜치로 구성:

```
<시나리오 레포>/
  codrill.toml   # 메타데이터
  BRIEF.md       # main 브랜치에만 존재 -- 상황 설명
  README.md      # 이 가짜 프로젝트의 구조 설명
  verify/run     # 채점 스크립트 (실행 권한 필수, exit 0 = 통과)
  (버그 있는 코드)

  ── solution 브랜치 ──
  SOLUTION.md    # 여기만 존재 -- 정답 해설
  (고쳐진 코드, verify/run도 여기선 통과해야 함)
```

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
