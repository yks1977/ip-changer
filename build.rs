fn main() {
    println!("cargo:rerun-if-changed=assets/ip-changer.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        // 릴리스 exe는 실행할 때 관리자 권한을 요청한다 (IP 설정 변경에 필요).
        // 디버그 빌드(cargo test 포함)는 일반 권한으로 실행되게 둔다.
        let level = if std::env::var("PROFILE").as_deref() == Ok("release") {
            "requireAdministrator"
        } else {
            "asInvoker"
        };
        let manifest = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="IPChanger" version="1.0.0.0" processorArchitecture="*"/>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="{level}" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <!-- 배율을 직접 처리한다고 선언해서 Windows가 확대(흐릿하게 늘이기)하지 않게 한다.
           앱은 항상 100% 기준 픽셀 크기로 그린다. -->
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#
        );

        let mut resource = winres::WindowsResource::new();
        resource.set_icon("assets/ip-changer.ico");
        // set_manifest()는 각 줄 앞에 공백을 넣어 XML 선언이 깨지므로 파일로 넘긴다.
        let manifest_path =
            std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("app.manifest");
        std::fs::write(&manifest_path, manifest.trim_start()).unwrap();
        resource.set_manifest_file(manifest_path.to_str().unwrap());
        resource.set("FileDescription", "IP Changer");
        resource.set("ProductName", "IP Changer");
        resource.set("OriginalFilename", "IP Changer.exe");
        resource.set("FileVersion", env!("CARGO_PKG_VERSION"));
        resource.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        resource.set_language(0x0412); // 한국어

        if let Err(err) = resource.compile() {
            println!("cargo:warning=exe 리소스(아이콘·매니페스트)를 넣지 못했습니다: {err}");
        }
    }
}
