# IP Changer

Wi-Fi 어댑터의 IPv4 설정을 **고정 IP ↔ DHCP**로 빠르게 바꾸는 Windows 도구입니다.

- 프로필을 최대 3개까지 저장하고, 하단 버튼으로 바로 불러올 수 있습니다 (적용 버튼으로 반영).
- 지금 연결된 IP · 서브넷 마스크 · 게이트웨이 · DNS를 한눈에 보여줍니다.
- 한국어 / English 화면을 지원합니다 (⚙ 메뉴).

## 실행

`IP Changer.exe` 하나만 있으면 됩니다. 설치나 추가 런타임이 필요 없습니다.

- Windows 11 (x64)
- IP 설정을 바꾸려면 관리자 권한이 필요해서 실행할 때 UAC 창이 뜹니다.
- 화면은 100% 배율(FHD) 기준 크기로 고정되며, 배율이 다른 모니터로 옮겨도 크기가 바뀌지 않습니다.

설정과 프로필은 `C:\Users\<계정>\Documents\IP Changer\settings.json`에 저장됩니다.

## 빌드

Rust(MSVC 툴체인)가 필요합니다.

```bat
build.bat
```

`cargo build --release` 후 결과물을 `IP Changer.exe`로 복사합니다.

- 실행 파일에는 외부 크레이트가 들어가지 않습니다. 화면은 Win32 API(user32 / gdi32 / comctl32)를 직접 호출하고,
  네트워크 설정은 Windows PowerShell(`Set-NetIPInterface`, `New-NetIPAddress` 등)로 바꿉니다.
- `winres`는 빌드할 때 아이콘·매니페스트를 넣는 데만 씁니다.
- VC++ 런타임은 정적으로 링크합니다(`.cargo/config.toml`).

테스트:

```bat
cargo test
```
