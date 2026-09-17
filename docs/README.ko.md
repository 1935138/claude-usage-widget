<h1 align="center">Claude Usage Widget</h1>

<p align="center">
  Claude Code 사용량을 항상 화면에 띄워두는 위젯
</p>

<p align="center">
  <a href="https://github.com/1935138/claude-usage-widget/releases"><img alt="Release" src="https://img.shields.io/github/v/release/1935138/claude-usage-widget"></a>
  <a href="https://github.com/1935138/claude-usage-widget/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/1935138/claude-usage-widget/total"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-blue">
  <a href="../LICENSE"><img alt="License" src="https://img.shields.io/github/license/1935138/claude-usage-widget"></a>
</p>

<p align="center">
  <img src="screenshot.png" alt="세션 미터와 주간 미터 두 개를 보여주는 위젯" width="400">
</p>

<p align="center">
  <a href="../README.md">English</a> · 한국어
</p>

`/usage`는 물어볼 때만 답을 줍니다. 이 위젯은 같은 수치를 계속 띄워둡니다. 5시간
세션 창, 주간 한도, 모델별 주간 한도, 그리고 각각의 초기화 시각까지 함께
보여줍니다.

## 기능

- **실시간 수치.** Claude Code가 쓰는 사용량 엔드포인트를 그대로 읽습니다.
  마지막으로 `/usage`를 친 시점의 캐시가 아니라 지금 값입니다.
- **페이스 마커.** 막대 위의 빨간 선은 시간이 어디까지 흘렀는지를 나타냅니다.
  막대가 마커보다 앞서 있으면 초기화 전에 한도를 다 쓰게 됩니다.
- **Windows와 WSL 모두.** WSL 배포판 안에 설치된 것까지 포함해, 이 컴퓨터의 모든
  Claude Code 설치본을 찾습니다.
- **여러 계정.** 한도는 계정 단위입니다. 설치본들이 서로 다른 계정으로 로그인되어
  있으면 드롭다운으로 골라 볼 수 있습니다.
- **로그인 안 되어 있으면 버튼 하나로.** 자격 증명이 없으면 `claude login`을
  실행하는 버튼을 대신 보여줍니다.
- **거슬리지 않게.** 항상 위에 뜨고, 작업 표시줄에 남지 않고, 내용에 맞춰 창
  크기를 조절하며, Windows 라이트/다크 테마를 따릅니다.

## 설치

설치 파일을 받아 실행하면 됩니다.

| 파일 | 대상 |
| --- | --- |
| [claude-usage-widget_0.1.1_x64-setup.exe](https://github.com/1935138/claude-usage-widget/releases/download/v0.1.1/claude-usage-widget_0.1.1_x64-setup.exe) | 64비트 Windows |
| [claude-usage-widget_0.1.1_x86-setup.exe](https://github.com/1935138/claude-usage-widget/releases/download/v0.1.1/claude-usage-widget_0.1.1_x86-setup.exe) | 32비트 Windows |

Windows 11에는 WebView2 런타임이 이미 들어 있습니다. Windows 10이면 설치 과정에서
필요할 때 받아옵니다. 이전 버전과 `SHA256SUMS`는
[릴리스 페이지](https://github.com/1935138/claude-usage-widget/releases)에 있습니다.

Claude Code가 설치되어 있고 로그인되어 있어야 합니다. 그 외에 설정할 것은 없습니다.

## 사용법

제목 표시줄을 잡고 드래그하면 창이 움직이고, ✕로 닫습니다. 슬라이더 아이콘을
누르면 설정 패널이 열립니다.

| 설정 | 선택지 |
| --- | --- |
| 표시할 미터 | 현재 세션, 주간 한도 |
| 캐시·크레딧 정보 | 켜기 / 끄기 |
| 페이스 마커 | 켜기 / 끄기 |
| 갱신 주기 | 수동, 3분 ~ 1시간 (기본 5분) |
| 시계 | 표시 / 숨김 |
| 모서리 | 각지게, 살짝, 둥글게(기본), 많이 둥글게 |

설정은 앱 설정 디렉터리의 `settings.json`에 저장되며 직접 편집해도 됩니다. 범위를
벗어난 값은 파일을 초기화하는 대신 불러올 때 교정되고, UTF-8 BOM이 붙어 있어도
읽습니다. `cornerRadius`는 패널에 없는 값이라도 24px까지 그대로 적용됩니다.

창에는 장식이 없고 배경이 투명합니다. Windows가 그려주는 테두리가 없기 때문에
카드가 스스로 외곽선을 그리며, **모서리** 설정이 그 모양을 결정합니다.

## 동작 방식

### 수치의 출처

퍼센트 값은 Claude Code의 세션 로그에서 계산할 수 있는 값이 아닙니다.
Claude Code가 직접 호출하는 엔드포인트인
`GET https://api.anthropic.com/api/oauth/usage`에서 가져오며,
`~/.claude/.credentials.json`에 저장된 OAuth 액세스 토큰으로 인증합니다.

Claude Code는 마지막 응답을 `~/.claude.json`의 `cachedUsageUtilization`에
캐시해 두는데, 실시간 조회가 불가능할 때는 위젯도 이 값으로 대체합니다. 캐시는
어디까지나 폴백입니다. 자격 증명만 살아 있으면 그 설치본에서 `/usage`를 한 번도
돌린 적이 없어도 실시간 수치가 나옵니다. 캐시 값을 보여줄 때는 이유도 함께
적습니다. 로그인 만료인지, 레이트 리밋인지, 응답이 없었는지 구분해 줍니다.

이 엔드포인트는 문서화된 공식 API가 아닙니다. 사양이 바뀌면 위젯은 조용히 캐시
값으로 물러납니다.

### 자격 증명

**리프레시 토큰은 읽지도 쓰지도 않습니다.** 이 토큰은 Claude Code가 회전시키는
값이라, 다른 프로세스가 파일에 쓰면 Claude Code 쪽 로그인이 풀릴 수 있습니다.
액세스 토큰은 있는 그대로만 쓰고, 만료되면 Claude Code가 평소 사용 중에 갱신할
때까지 캐시 값을 보여줍니다.

위에 적은 요청 하나를 빼면 이 컴퓨터 밖으로 나가는 것은 없습니다. 오류 값에는
자격 증명 파일의 내용이 담기지 않으므로 토큰이 로그나 화면에 새어 나갈 일이
없습니다. 같은 이유로 로그인도 위젯이 직접 처리하지 않고 별도 콘솔의
`claude login`에 맡깁니다.

### 설치본 탐색

다음 순서로 찾습니다.

1. `CLAUDE_CONFIG_DIR` (설정되어 있으면. 쉼표나 세미콜론으로 여러 개 지정 가능)
2. 사용자 홈 디렉터리의 `.claude`와 `.config/claude`
3. 설치된 모든 WSL 배포판. 네트워크 공유는 *실행 중인* 배포판만 보여주기 때문에
   이름은 `Lxss` 레지스트리 키에서 읽습니다. `\\wsl.localhost`를 먼저 시도하고
   안 되면 `\\wsl$`로 넘어갑니다.

같은 계정으로 로그인된 설치본이 여러 개여도 목록에는 하나만 나옵니다. 같은
로그인이라도 설치본마다 `account_uuid`가 다를 수 있어서 이메일을 기준으로
묶습니다. 계정 정보는 캐시 블록이 아니라 `oauthAccount`에서 읽습니다. 캐시에 적힌
계정은 그 캐시를 마지막으로 쓴 시점의 로그인이라 지금과 다를 수 있습니다.

## 빌드

MSVC 빌드 도구, WebView2, Node, Rust(`winget install Rustlang.Rustup`)가
필요합니다.

```sh
npm install
npm run tauri dev                # 라이브 리로드 창
npm run tauri build              # 설치 파일
npm run tauri build --no-bundle  # 실행 파일만
```

알아두면 좋은 세 가지입니다.

- 프로젝트는 Windows 파일 시스템에 두세요. Windows `node.exe`는 `/home/...`
  경로를 해석하지 못하고, `\\wsl.localhost` 너머로 cargo를 돌리면 못 쓸 만큼
  느립니다.
- `cargo build`를 직접 쓰지 말고 Tauri CLI로 빌드하세요. `tauri-build`는 CLI가
  아니라고 알려주지 않으면 `cfg(dev)`를 켜기 때문에, 그냥
  `cargo build --release`로 만든 바이너리는 여전히 dev 서버를 바라봅니다.
- Windows 셸에서 실행하세요. WSL 셸에서는 Windows Tauri CLI가 리눅스 `PATH`를
  물려받아 `cargo metadata ... program not found`로 실패합니다.
- 빌드 전에 위젯을 종료하세요. Windows는 실행 중인 `.exe`를 잠그기 때문에 링크
  단계에서 `failed to remove file ... Access is denied. (os error 5)`로 실패합니다.

### 리눅스에서 크로스 컴파일

```sh
rustup target add x86_64-pc-windows-msvc i686-pc-windows-msvc
cargo install --locked cargo-xwin
sudo apt install llvm clang lld     # llvm-rc, clang-cl, lld-link

export XWIN_ACCEPT_LICENSE=1
export XWIN_ARCH=x86_64,x86         # x86 import lib이 없으면 32비트 링크 실패
npm install
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

`npm run tauri dev`는 창을 띄우는 명령이라 Windows에서만 동작합니다. 크로스
빌드에는 `CARGO_TARGET_DIR`로 별도 디렉터리를 주세요. 그러지 않으면 호스트
빌드와 `target/debug`를 두고 다툽니다.

### 점검

```sh
cargo test -p claude-usage-core
cargo run -p claude-usage-core --bin probe           # 위젯이 보여줄 내용
cargo run -p claude-usage-core --bin probe -- --json # UI가 받는 그대로
npm run build                                        # 타입, 번들, CSS 검사
```

`npm run build`는 마지막에 CSS가 끝까지 압축됐는지 확인합니다. 규칙 하나가
깨져 있으면 esbuild가 그 뒤를 원문 그대로 흘려보내는데, 빌드는 성공으로 끝나면서
이후 규칙이 전부 사라집니다.

## 구조

```
core/        순수 Rust: 설치본 탐색, 사용량 API 클라이언트, 설정.
             Tauri에 의존하지 않아 어디서든 빌드·테스트됩니다.
core/src/bin/probe.rs   창 없이 위젯이 보여줄 내용을 출력합니다.
src-tauri/   Tauri 2 셸: 커맨드와 창 크기 조절.
src/         UI (TypeScript + Vite, 프레임워크 없음).
```

`core`를 Tauri에서 떼어 둔 것은 의도한 것입니다. Tauri는 리눅스에서 GTK와 dbus를
끌어오는데, 그러면 로직 테스트를 제대로 갖춰진 Windows 환경에서만 돌릴 수 있게
됩니다.

## 색상

배경과 글자색은 Anthropic 브랜드 팔레트를 씁니다. 막대는 아니었습니다. 브랜드
강조색끼리는 색각 이상 검사를 통과하지 못해서, 브랜드 클레이에 가장 가까운
주황을 앞세운 검증된 색상을 대신 씁니다. 색은 항목 구분에만 씁니다. 막대가
얼마나 찼든 색은 그대로이고, 한도에 얼마나 가까운지는 퍼센트 숫자와 페이스
마커가 알려줍니다.

## 라이선스

MIT. [LICENSE](../LICENSE)를 참고하세요.
