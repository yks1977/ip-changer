//! 설정·프로필 저장 (`<Documents>\IP Changer\settings.json`).

use crate::json::Json;
use crate::net::{parse_subnet, prefix_to_mask};
use std::path::{Path, PathBuf};

/// 설정과 프로필을 저장하는 폴더 이름. `<Documents>\IP Changer`
pub const STORE_DIR_NAME: &str = "IP Changer";
/// 이름을 바꾸기 전에 쓰던 폴더. 있으면 새 폴더로 옮겨서 그대로 이어 쓴다.
const LEGACY_STORE_DIR_NAMES: [&str; 1] = ["wifi-ip-setter"];

// ------------------------------------------------------------
// 설정 / 프로필
// ------------------------------------------------------------
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Settings {
    /// "ko" 또는 "en"
    pub language: String,
    pub adapter: String,
    pub use_static: bool,
    pub ip: String,
    /// 서브넷 마스크 (예: 255.255.255.0)
    pub subnet: String,
    /// 구버전 settings.json의 "/bit" 값. 읽기만 하고 저장하지 않는다.
    pub prefix: String,
    pub gateway: String,
    pub dns1: String,
    pub dns2: String,
    /// 마지막으로 선택한 프로필 이름
    pub active_profile: String,
    /// 프로필은 별도 파일이 아니라 settings.json 안에 함께 저장한다.
    pub profiles: Vec<Profile>,
}

impl Settings {
    pub fn default_config() -> Self {
        Self {
            language: "ko".to_string(),
            use_static: true,
            ip: "192.168.0.100".to_string(),
            subnet: "255.255.255.0".to_string(),
            gateway: "192.168.0.1".to_string(),
            dns1: "8.8.8.8".to_string(),
            dns2: "8.8.4.4".to_string(),
            ..Default::default()
        }
    }

    pub fn from_json(value: &Json) -> Self {
        Self {
            language: value.str_of("language"),
            adapter: value.str_of("adapter"),
            use_static: value.bool_of("use_static", false),
            ip: value.str_of("ip"),
            subnet: value.str_of("subnet"),
            prefix: value.str_of("prefix"),
            gateway: value.str_of("gateway"),
            dns1: value.str_of("dns1"),
            dns2: value.str_of("dns2"),
            active_profile: value.str_of("active_profile"),
            profiles: value
                .list_of("profiles")
                .iter()
                .filter(|item| matches!(item, Json::Obj(_)))
                .map(Profile::from_json)
                .collect(),
        }
    }

    pub fn to_json(&self) -> Json {
        let s = |v: &str| Json::Str(v.to_string());
        Json::Obj(vec![
            ("language".into(), s(&self.language)),
            ("adapter".into(), s(&self.adapter)),
            ("use_static".into(), Json::Bool(self.use_static)),
            ("ip".into(), s(&self.ip)),
            ("subnet".into(), s(&self.subnet)),
            ("gateway".into(), s(&self.gateway)),
            ("dns1".into(), s(&self.dns1)),
            ("dns2".into(), s(&self.dns2)),
            ("active_profile".into(), s(&self.active_profile)),
            (
                "profiles".into(),
                Json::Arr(self.profiles.iter().map(Profile::to_json).collect()),
            ),
        ])
    }

    /// 구버전의 "/bit" 값을 점 표기 서브넷 마스크로 옮긴다.
    pub fn migrate_subnet(&mut self) {
        self.subnet = parse_subnet(&self.subnet)
            .or_else(|| parse_subnet(&self.prefix))
            .map(prefix_to_mask)
            .unwrap_or_else(|| "255.255.255.0".to_string());
        self.prefix.clear();
        if self.language != "en" {
            self.language = "ko".to_string();
        }
    }

    pub fn sort_profiles(&mut self) {
        self.profiles
            .sort_by_key(|profile| profile.name.to_lowercase());
    }
}

/// 이름이 붙은 설정 묶음. settings.json의 `profiles` 배열에 함께 저장된다.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    pub adapter: String,
    pub use_static: bool,
    pub ip: String,
    pub subnet: String,
    pub gateway: String,
    pub dns1: String,
    pub dns2: String,
}

impl Profile {
    pub fn from_json(value: &Json) -> Self {
        Self {
            name: value.str_of("name"),
            adapter: value.str_of("adapter"),
            use_static: value.bool_of("use_static", false),
            ip: value.str_of("ip"),
            subnet: value.str_of("subnet"),
            gateway: value.str_of("gateway"),
            dns1: value.str_of("dns1"),
            dns2: value.str_of("dns2"),
        }
    }

    pub fn to_json(&self) -> Json {
        let s = |v: &str| Json::Str(v.to_string());
        Json::Obj(vec![
            ("name".into(), s(&self.name)),
            ("adapter".into(), s(&self.adapter)),
            ("use_static".into(), Json::Bool(self.use_static)),
            ("ip".into(), s(&self.ip)),
            ("subnet".into(), s(&self.subnet)),
            ("gateway".into(), s(&self.gateway)),
            ("dns1".into(), s(&self.dns1)),
            ("dns2".into(), s(&self.dns2)),
        ])
    }

    pub fn from_settings(name: &str, settings: &Settings) -> Self {
        Self {
            name: name.to_string(),
            adapter: settings.adapter.clone(),
            use_static: settings.use_static,
            ip: settings.ip.clone(),
            subnet: normalize_subnet(&settings.subnet),
            gateway: settings.gateway.clone(),
            dns1: settings.dns1.clone(),
            dns2: settings.dns2.clone(),
        }
    }

    pub fn apply_to(&self, settings: &mut Settings) {
        settings.adapter = self.adapter.clone();
        settings.use_static = self.use_static;
        settings.ip = self.ip.clone();
        settings.subnet = normalize_subnet(&self.subnet);
        settings.gateway = self.gateway.clone();
        settings.dns1 = self.dns1.clone();
        settings.dns2 = self.dns2.clone();
    }
}

/// "24" 같은 비트 표기가 남아 있으면 점 표기 마스크로 맞춘다.
pub fn normalize_subnet(text: &str) -> String {
    match parse_subnet(text) {
        Some(bits) => prefix_to_mask(bits),
        None => text.trim().to_string(),
    }
}

/// 프로필 이름을 검사한다. 오류 시 tr() 키를 돌려준다.
pub fn validate_profile_name(name: &str) -> Result<String, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("profile_name_empty");
    }
    if name.chars().count() > 40 {
        return Err("profile_name_too_long");
    }
    if name.chars().any(|ch| {
        ch.is_control() || matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) || name.starts_with('.')
        || name.ends_with('.')
    {
        return Err("profile_name_invalid");
    }
    // Windows 예약 이름은 파일로 만들 수 없다 (예전 버전은 프로필을 파일로 저장했다).
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    if reserved.contains(&stem.as_str()) {
        return Err("profile_name_invalid");
    }
    Ok(name.to_string())
}

// ------------------------------------------------------------
// 저장 위치: C:\Users\<계정>\Documents\IP Changer
// ------------------------------------------------------------
fn write_atomic(path: &Path, data: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

/// 설정을 저장할 문서 폴더. 항상 로그인 계정의 `C:\Users\<계정>\Documents`를 쓴다.
///
/// 셸의 "문서" 폴더는 OneDrive로 리디렉션되어 있을 수 있다(예: `OneDrive\문서`).
/// OneDrive를 쓰지 않는 PC와 저장 경로를 맞추기 위해 사용자 폴더 아래의 `Documents`를 직접 쓴다.
pub fn documents_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        let docs = home.join("Documents");
        if docs.is_dir() || std::fs::create_dir_all(&docs).is_ok() {
            return docs;
        }
        return home;
    }
    win_path::documents().unwrap_or_else(std::env::temp_dir)
}

fn local_app_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(STORE_DIR_NAME)
}

/// 구버전이 설정을 저장하던 위치들
/// (exe 옆, LOCALAPPDATA, OneDrive로 리디렉션됐던 문서 폴더, 옛 폴더 이름).
fn legacy_settings_paths() -> Vec<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let docs = documents_dir();
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let mut paths = vec![
        exe_dir.join("settings.json"),
        local.join(STORE_DIR_NAME).join("settings.json"),
        local.join(LEGACY_STORE_DIR_NAMES[0]).join("settings.json"),
    ];
    let shell_docs = win_path::documents();
    for old in LEGACY_STORE_DIR_NAMES {
        paths.push(docs.join(old).join("settings.json"));
        if let Some(shell_docs) = &shell_docs {
            if shell_docs != &docs {
                paths.push(shell_docs.join(old).join("settings.json"));
            }
        }
    }
    paths.dedup();
    paths
}

#[cfg(windows)]
mod win_path {
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;

    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHGetKnownFolderPath(
            rfid: *const Guid,
            dwflags: u32,
            htoken: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoTaskMemFree(pv: *mut c_void);
    }

    // {FDD39AD0-238F-46AF-ADB4-6C85480369C7}
    const FOLDERID_DOCUMENTS: Guid = Guid {
        data1: 0xFDD3_9AD0,
        data2: 0x238F,
        data3: 0x46AF,
        data4: [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7],
    };

    /// 현재 사용자의 "문서" 폴더. 실패하면 None.
    pub fn documents() -> Option<PathBuf> {
        let mut raw: *mut u16 = std::ptr::null_mut();
        let hr =
            unsafe { SHGetKnownFolderPath(&FOLDERID_DOCUMENTS, 0, std::ptr::null_mut(), &mut raw) };
        if hr < 0 || raw.is_null() {
            return None;
        }
        let mut len = 0usize;
        unsafe {
            while *raw.add(len) != 0 {
                len += 1;
            }
        }
        let wide = unsafe { std::slice::from_raw_parts(raw, len) };
        let path = PathBuf::from(OsString::from_wide(wide));
        unsafe { CoTaskMemFree(raw.cast()) };
        Some(path)
    }
}

#[cfg(not(windows))]
mod win_path {
    pub fn documents() -> Option<std::path::PathBuf> {
        None
    }
}

/// 설정 폴더 경로를 정한다. 이름을 바꾸기 전 폴더가 남아 있으면 통째로 이름을 바꿔 이어 쓴다.
fn store_root(docs: &Path) -> PathBuf {
    let preferred = docs.join(STORE_DIR_NAME);
    for old_name in LEGACY_STORE_DIR_NAMES {
        let old = docs.join(old_name);
        if !old.is_dir() {
            continue;
        }
        // 새 폴더가 이미 있으면 덮어쓰지 않는다(설정은 legacy 경로에서 가져온다).
        if preferred.exists() {
            break;
        }
        if std::fs::rename(&old, &preferred).is_ok() {
            break;
        }
    }
    preferred
}

/// 설정 파일을 읽고 쓴다.
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// 문서 폴더를 우선 사용하고, 쓸 수 없으면 LOCALAPPDATA로 대체한다.
    pub fn open() -> Self {
        let preferred = store_root(&documents_dir());
        if std::fs::create_dir_all(&preferred).is_ok() {
            return Self { root: preferred };
        }
        let fallback = local_app_data_dir();
        let _ = std::fs::create_dir_all(&fallback);
        Self { root: fallback }
    }

    #[cfg(test)]
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    pub fn load_settings(&self) -> Settings {
        if let Some(mut settings) = read_settings(&self.settings_path()) {
            settings.migrate_subnet();
            if self.import_legacy_profiles(&mut settings) {
                self.save_settings(&settings);
            }
            settings.sort_profiles();
            return settings;
        }

        // 구버전 위치 중 가장 최근 설정을 쓴다.
        let legacy = legacy_settings_paths()
            .into_iter()
            .filter_map(|path| {
                let modified = std::fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .ok()?;
                let settings = read_settings(&path)?;
                Some((modified, path, settings))
            })
            .max_by_key(|(modified, _, _)| *modified);

        if let Some((_, path, mut settings)) = legacy {
            settings.migrate_subnet();
            if let Some(root) = path.parent() {
                import_profile_files(root, &mut settings);
            }
            self.import_legacy_profiles(&mut settings);
            settings.sort_profiles();
            self.save_settings(&settings);
            return settings;
        }

        let mut settings = Settings::default_config();
        self.import_legacy_profiles(&mut settings);
        self.save_settings(&settings);
        settings
    }

    /// 설정을 settings.json에 쓴다. 성공하면 true.
    pub fn save_settings(&self, settings: &Settings) -> bool {
        let data = settings.to_json().to_pretty();
        if write_atomic(&self.settings_path(), &data).is_ok() {
            return true;
        }
        // 문서 폴더에 쓸 수 없으면 LOCALAPPDATA에라도 남긴다.
        write_atomic(&local_app_data_dir().join("settings.json"), &data).is_ok()
    }

    /// 예전 버전이 `profiles\<이름>.json`에 저장하던 프로필을 settings.json 안으로 옮긴다.
    fn import_legacy_profiles(&self, settings: &mut Settings) -> bool {
        let imported = import_profile_files(&self.root, settings);
        self.retire_legacy_profiles();
        imported
    }

    fn retire_legacy_profiles(&self) {
        let dir = self.profiles_dir();
        if !dir.is_dir() {
            return;
        }
        let backup = self.root.join("profiles.backup");
        if backup.exists() {
            let _ = std::fs::remove_dir_all(&backup);
        }
        let _ = std::fs::rename(&dir, &backup);
    }
}

fn read_json(path: &Path) -> Option<Json> {
    let data = std::fs::read_to_string(path).ok()?;
    let value = Json::parse(&data)?;
    matches!(value, Json::Obj(_)).then_some(value)
}

fn read_settings(path: &Path) -> Option<Settings> {
    read_json(path).map(|value| Settings::from_json(&value))
}

fn read_profile(path: &Path) -> Option<Profile> {
    let mut profile = Profile::from_json(&read_json(path)?);
    if profile.name.trim().is_empty() {
        profile.name = path.file_stem()?.to_str()?.to_string();
    }
    Some(profile)
}

/// `<root>\profiles\*.json`에 남아 있던 예전 프로필 파일을 settings.profiles로 합친다.
fn import_profile_files(root: &Path, settings: &mut Settings) -> bool {
    let dir = root.join("profiles");
    if !dir.is_dir() {
        return false;
    }
    let mut imported = false;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_json = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
            if !is_json {
                continue;
            }
            let Some(mut profile) = read_profile(&path) else {
                continue;
            };
            let Ok(name) = validate_profile_name(&profile.name) else {
                continue;
            };
            profile.name = name;
            profile.subnet = normalize_subnet(&profile.subnet);
            if settings
                .profiles
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(&profile.name))
            {
                continue;
            }
            settings.profiles.push(profile);
            imported = true;
        }
    }
    imported
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> (Store, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("ip-changer-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Store::at(dir.clone()), dir)
    }

    #[test]
    fn old_prefix_setting_is_migrated_to_mask() {
        let mut settings =
            Settings::from_json(&Json::parse(r#"{"ip":"10.0.0.5","prefix":"24"}"#).unwrap());
        settings.migrate_subnet();
        assert_eq!(settings.subnet, "255.255.255.0");
        assert!(settings.prefix.is_empty());

        let mut settings = Settings::from_json(
            &Json::parse(r#"{"subnet":"255.255.0.0","prefix":"16"}"#).unwrap(),
        );
        settings.migrate_subnet();
        assert_eq!(settings.subnet, "255.255.0.0");

        let mut settings = Settings::default();
        settings.migrate_subnet();
        assert_eq!(settings.subnet, "255.255.255.0");

        // 저장 시에는 더 이상 "prefix"를 쓰지 않는다.
        let json = Settings::default_config().to_json().to_pretty();
        assert!(json.contains("\"subnet\": \"255.255.255.0\""));
        assert!(!json.contains("\"prefix\""));
    }

    #[test]
    fn reads_settings_written_by_previous_versions() {
        // serde_json으로 저장하던 이전 버전의 settings.json 형식
        let old = r#"{
  "language": "en",
  "adapter": "Wi-Fi",
  "use_static": true,
  "ip": "10.0.0.5",
  "subnet": "255.255.255.0",
  "gateway": "10.0.0.1",
  "dns1": "1.1.1.1",
  "dns2": "",
  "active_profile": "집",
  "profiles": [
    {"name": "집", "adapter": "Wi-Fi", "use_static": false, "ip": "", "subnet": "", "gateway": "", "dns1": "", "dns2": ""}
  ]
}"#;
        let settings = Settings::from_json(&Json::parse(old).unwrap());
        assert_eq!(settings.language, "en");
        assert!(settings.use_static);
        assert_eq!(settings.gateway, "10.0.0.1");
        assert_eq!(settings.profiles.len(), 1);
        assert_eq!(settings.profiles[0].name, "집");
        assert!(!settings.profiles[0].use_static);
        // 다시 써도 같은 값으로 읽힌다.
        let again = Settings::from_json(&Json::parse(&settings.to_json().to_pretty()).unwrap());
        assert_eq!(again, settings);
    }

    #[test]
    fn profile_names_are_validated() {
        assert_eq!(validate_profile_name("  사무실  "), Ok("사무실".to_string()));
        assert_eq!(validate_profile_name("Office-1"), Ok("Office-1".to_string()));
        assert_eq!(validate_profile_name("집.와이파이"), Ok("집.와이파이".to_string()));
        assert_eq!(validate_profile_name(""), Err("profile_name_empty"));
        assert_eq!(validate_profile_name("   "), Err("profile_name_empty"));
        assert_eq!(validate_profile_name("a/b"), Err("profile_name_invalid"));
        assert_eq!(validate_profile_name("a\\b"), Err("profile_name_invalid"));
        assert_eq!(validate_profile_name("a:b"), Err("profile_name_invalid"));
        assert_eq!(validate_profile_name("CON"), Err("profile_name_invalid"));
        assert_eq!(validate_profile_name("com1.txt"), Err("profile_name_invalid"));
        assert_eq!(validate_profile_name(".hidden"), Err("profile_name_invalid"));
        assert_eq!(validate_profile_name("trailing."), Err("profile_name_invalid"));
        assert_eq!(
            validate_profile_name(&"x".repeat(41)),
            Err("profile_name_too_long")
        );
    }

    #[test]
    fn profiles_are_stored_inside_settings_json() {
        let (store, dir) = temp_store("profiles");

        let mut settings = Settings::default_config();
        settings.adapter = "Wi-Fi".to_string();
        settings.ip = "10.0.0.5".to_string();
        settings.gateway = "10.0.0.1".to_string();
        settings.dns1 = "1.1.1.1".to_string();
        settings.dns2 = String::new();
        settings
            .profiles
            .push(Profile::from_settings("사무실", &settings));
        settings.active_profile = "사무실".to_string();
        assert!(store.save_settings(&settings));

        assert!(!store.profiles_dir().exists());
        let raw = std::fs::read_to_string(store.settings_path()).unwrap();
        assert!(raw.contains("\"profiles\""));
        assert!(raw.contains("사무실"));

        let loaded = store.load_settings();
        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.profiles[0].name, "사무실");
        assert_eq!(loaded.profiles[0].ip, "10.0.0.5");
        assert_eq!(loaded.profiles[0].subnet, "255.255.255.0");
        assert!(loaded.profiles[0].dns2.is_empty());
        assert_eq!(loaded.active_profile, "사무실");

        let mut target = Settings::default_config();
        loaded.profiles[0].apply_to(&mut target);
        assert_eq!(target.adapter, "Wi-Fi");
        assert_eq!(target.ip, "10.0.0.5");
        assert_eq!(target.gateway, "10.0.0.1");
        assert!(target.dns2.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_profile_files_move_into_settings_json() {
        let (store, dir) = temp_store("legacy-profile");
        assert!(store.save_settings(&Settings::default_config()));

        let legacy = Profile {
            name: "옛 프로필".to_string(),
            adapter: "Wi-Fi".to_string(),
            use_static: true,
            ip: "172.16.0.10".to_string(),
            subnet: "24".to_string(),
            gateway: "172.16.0.1".to_string(),
            dns1: "1.1.1.1".to_string(),
            dns2: String::new(),
        };
        std::fs::create_dir_all(store.profiles_dir()).unwrap();
        std::fs::write(
            store.profiles_dir().join("legacy.json"),
            legacy.to_json().to_pretty(),
        )
        .unwrap();

        let settings = store.load_settings();
        assert_eq!(settings.profiles.len(), 1);
        assert_eq!(settings.profiles[0].name, "옛 프로필");
        assert_eq!(settings.profiles[0].subnet, "255.255.255.0");

        assert!(!store.profiles_dir().exists());
        assert!(dir.join("profiles.backup").is_dir());
        assert_eq!(store.load_settings().profiles.len(), 1);

        let raw = std::fs::read_to_string(store.settings_path()).unwrap();
        assert!(raw.contains("172.16.0.10"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn documents_folder_is_resolved() {
        let docs = documents_dir();
        assert!(docs.is_absolute());
        assert!(!docs.to_string_lossy().contains("OneDrive"));
    }

    #[test]
    fn store_folder_renames_legacy_directory() {
        let base =
            std::env::temp_dir().join(format!("ip-changer-test-{}-rename", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("wifi-ip-setter")).unwrap();
        std::fs::write(base.join("wifi-ip-setter").join("settings.json"), "{}").unwrap();

        let root = store_root(&base);
        assert_eq!(root, base.join(STORE_DIR_NAME));
        assert!(root.join("settings.json").is_file());
        assert!(!base.join("wifi-ip-setter").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn existing_store_folder_is_not_overwritten() {
        let base =
            std::env::temp_dir().join(format!("ip-changer-test-{}-keep", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("wifi-ip-setter")).unwrap();
        std::fs::create_dir_all(base.join(STORE_DIR_NAME)).unwrap();
        std::fs::write(base.join(STORE_DIR_NAME).join("settings.json"), "{\"ip\":\"1\"}")
            .unwrap();

        store_root(&base);
        assert!(base.join("wifi-ip-setter").exists());
        assert_eq!(
            std::fs::read_to_string(base.join(STORE_DIR_NAME).join("settings.json")).unwrap(),
            "{\"ip\":\"1\"}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
