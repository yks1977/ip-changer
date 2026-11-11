//! 네트워크 어댑터 조회, 입력 검증, PowerShell로 IP 설정 적용.

use crate::json::Json;
use std::process::Command;

/// Wi-Fi 어댑터 정보 (PowerShell이 JSON으로 돌려준 값).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdapterInfo {
    /// 네트워크 연결 이름 (예: "Wi-Fi")
    pub name: String,
    /// 실제 장치 이름
    pub description: String,
    pub wifi: bool,
    /// "up" / "disconnected" / "down"
    pub status: String,
    pub ip: Option<String>,
    pub mask: Option<String>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    pub dhcp: bool,
}

impl AdapterInfo {
    fn from_json(value: &Json) -> Self {
        let opt = |key: &str| Some(value.str_of(key)).filter(|v| !v.trim().is_empty());
        Self {
            name: value.str_of("name"),
            description: value.str_of("description"),
            wifi: value.bool_of("wifi", false),
            status: value.str_of("status"),
            ip: opt("ip"),
            mask: opt("mask"),
            gateway: opt("gateway"),
            dns: value
                .list_of("dns")
                .iter()
                .filter_map(|item| match item {
                    Json::Str(s) if !s.trim().is_empty() => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            dhcp: value.bool_of("dhcp", false),
        }
    }

    #[cfg(test)]
    pub fn prefix(&self) -> Option<u8> {
        self.mask.as_deref().and_then(mask_to_prefix)
    }
}

/// Windows PowerShell 5.1은 항목이 1개면 배열 대신 객체를 돌려줄 수 있어 둘 다 처리한다.
/// Wi-Fi 어댑터만 남긴다.
pub fn parse_adapters(json: &str) -> Vec<AdapterInfo> {
    let items = match Json::parse(json.trim()) {
        Some(Json::Arr(items)) => items,
        Some(value @ Json::Obj(_)) => vec![value],
        _ => vec![],
    };
    items
        .iter()
        .filter(|item| matches!(item, Json::Obj(_)))
        .map(AdapterInfo::from_json)
        .filter(|adapter| !adapter.name.is_empty())
        .filter(|adapter| adapter.wifi)
        .filter(|adapter| {
            let name = adapter.name.to_lowercase();
            let desc = adapter.description.to_lowercase();
            !(name.contains("bluetooth") || desc.contains("bluetooth"))
        })
        .collect()
}

#[cfg(windows)]
pub fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub fn hide_console(_cmd: &mut Command) {}

// ------------------------------------------------------------
// PowerShell 실행 헬퍼
// ------------------------------------------------------------
pub fn run_powershell(script: &str) -> Result<String, String> {
    let script = format!(
        "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; \
         $OutputEncoding=[System.Text.Encoding]::UTF8; \
         $ProgressPreference='SilentlyContinue'; {script}"
    );

    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &script,
    ]);
    hide_console(&mut cmd);

    let output = cmd.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("PowerShell exited with {}", output.status)
        })
    }
}


pub fn mask_to_prefix(mask: &str) -> Option<u8> {
    let value = ipv4_to_u32(mask)?;
    let bits = value.leading_ones() as u8;
    let expected = if bits == 0 {
        0
    } else {
        u32::MAX << (32 - bits)
    };
    if expected == value {
        Some(bits)
    } else {
        None
    }
}

/// prefix 비트 수를 점 표기 서브넷 마스크로 바꾼다 (24 → "255.255.255.0").
pub fn prefix_to_mask(bits: u8) -> String {
    let bits = bits.min(32);
    let mask = if bits == 0 {
        0
    } else {
        u32::MAX << (32 - bits)
    };
    let [a, b, c, d] = mask.to_be_bytes();
    format!("{a}.{b}.{c}.{d}")
}

/// 서브넷 입력을 prefix 비트 수로 바꾼다.
/// "255.255.255.0" 같은 점 표기와 "/24", "24" 같은 비트 표기를 모두 받는다.
pub fn parse_subnet(text: &str) -> Option<u8> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if text.contains('.') {
        mask_to_prefix(text)
    } else {
        text.trim_start_matches('/')
            .parse::<u8>()
            .ok()
            .filter(|bits| *bits <= 32)
    }
}


/// 실제 물리 네트워크 카드(유선/무선)의 별칭·장치명·현재 IP를 한 번에 가져온다.
pub fn list_adapters() -> Result<Vec<AdapterInfo>, String> {
    let script = r#"$ErrorActionPreference='SilentlyContinue'
$cfgs=@{}
Get-CimInstance Win32_NetworkAdapterConfiguration | ForEach-Object { $cfgs[[int]$_.InterfaceIndex]=$_ }
  $items=@(Get-CimInstance Win32_NetworkAdapter | Where-Object {
    $_.PhysicalAdapter -eq $true -and
    $_.NetConnectionID -and
    $_.NetConnectionID -notmatch 'Bluetooth' -and
    $_.Name -notmatch 'Bluetooth' -and
    $_.InterfaceType -ne 6
  } | ForEach-Object {
  $a=$_
  $c=$cfgs[[int]$a.InterfaceIndex]
  $v4=@()
  $masks=@()
  if ($c -and $c.IPAddress) {
    $ips=@($c.IPAddress)
    $subs=@($c.IPSubnet)
    for ($i=0; $i -lt $ips.Count; $i++) {
      if ($ips[$i] -notmatch ':') {
        $v4+=$ips[$i]
        if ($i -lt $subs.Count) { $masks+=$subs[$i] }
      }
    }
  }
  $dns=@()
  if ($c -and $c.DNSServerSearchOrder) { $dns=@($c.DNSServerSearchOrder) | Where-Object { $_ -notmatch ':' } }
  $gw=@()
  if ($c -and $c.DefaultIPGateway) { $gw=@($c.DefaultIPGateway) | Where-Object { $_ -notmatch ':' } }
  $status='down'
  if ($a.NetConnectionStatus -eq 2) { $status='up' } elseif ($a.NetConnectionStatus -eq 7) { $status='disconnected' }
  $wifi=[bool](($a.Name -match 'Wi-Fi|Wireless|802\.11|WLAN') -or ($a.NetConnectionID -match 'Wi-Fi|무선|WLAN'))
  [pscustomobject]@{
    name=[string]$a.NetConnectionID
    description=[string]$a.Name
    wifi=$wifi
    status=$status
    ip=[string]($v4 | Select-Object -First 1)
    mask=[string]($masks | Select-Object -First 1)
    gateway=[string]($gw | Select-Object -First 1)
    dns=@($dns)
    dhcp=[bool]$c.DHCPEnabled
  }
})
ConvertTo-Json -InputObject $items -Compress -Depth 4"#;

    let output = run_powershell(script)?;
    let output = output.trim();

    if output.is_empty() {
        return Err("PowerShell returned no data".to_string());
    }

    // Wi-Fi만 남기므로 유선만 있는 PC에서는 목록이 비는 게 정상이다.
    // JSON 자체가 깨졌을 때만 오류로 본다.
    if Json::parse(output).is_none() {
        return Err(format!(
            "unexpected adapter data: {}",
            truncate_chars(output, 200)
        ));
    }
    let mut adapters = parse_adapters(output);

    // 무선랜을 먼저 보여준다 (기본 선택도 무선랜).
    adapters.sort_by(|a, b| b.wifi.cmp(&a.wifi).then_with(|| a.name.cmp(&b.name)));
    Ok(adapters)
}


pub fn truncate_chars(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}


// ------------------------------------------------------------
// IPv4 유효성 검사
// ------------------------------------------------------------
pub fn ipv4_to_u32(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut value: u32 = 0;
    for part in parts {
        let octet: u8 = part.parse().ok()?;
        if part != octet.to_string() {
            return None; // 선행 0, 공백, 부호 등 거부
        }
        value = (value << 8) | u32::from(octet);
    }
    Some(value)
}

pub fn is_valid_ipv4(s: &str) -> bool {
    ipv4_to_u32(s).is_some()
}

/// 고정 IP 입력값을 검증하고 정규화된 prefix 비트 수를 돌려준다.
/// 오류 시에는 tr() 키를 반환한다.
pub fn validate_static(
    ip: &str,
    subnet: &str,
    gateway: &str,
    dns1: &str,
    dns2: &str,
) -> Result<u8, &'static str> {
    let ip_num = ipv4_to_u32(ip).ok_or("status_invalid_ip")?;

    // 0.0.0.0, 루프백, 멀티캐스트/예약 대역 거부
    if ip_num == 0 || (ip_num >> 24) == 127 || ip_num >= 0xE000_0000 {
        return Err("status_invalid_ip");
    }

    // 서브넷은 "255.255.255.0" 또는 "/24", "24" 모두 허용한다.
    let bits = match parse_subnet(subnet) {
        Some(bits @ 8..=32) => bits,
        _ => return Err("status_invalid_prefix"),
    };

    // /30 이하에서는 네트워크/브로드캐스트 주소를 호스트로 쓸 수 없다.
    if bits <= 30 {
        let mask = u32::MAX << (32 - bits);
        let network = ip_num & mask;
        if ip_num == network || ip_num == network | !mask {
            return Err("status_invalid_ip");
        }
    }

    if !gateway.is_empty() {
        let gw = ipv4_to_u32(gateway).ok_or("status_invalid_gateway")?;
        let mask = u32::MAX << (32 - bits);
        if gw == ip_num || (gw & mask) != (ip_num & mask) {
            return Err("status_invalid_gateway");
        }
    }

    for dns in [dns1, dns2] {
        if !dns.is_empty() && !is_valid_ipv4(dns) {
            return Err(if dns == dns1 {
                "status_invalid_dns1"
            } else {
                "status_invalid_dns2"
            });
        }
    }

    Ok(bits)
}

// ------------------------------------------------------------
// 설정 적용
// ------------------------------------------------------------
/// PowerShell 작은따옴표 문자열 내부용 이스케이프
pub fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

/// 적용 실패 시 이전 네트워크 설정을 되돌리는 공통 스크립트 조각.
/// format!에 값으로 삽입되므로 중괄호를 이스케이프하지 않는다.
pub const RESTORE_BLOCK: &str = r#"  try {
    Get-NetIPAddress -InterfaceAlias $alias -AddressFamily IPv4 -ErrorAction SilentlyContinue | Remove-NetIPAddress -Confirm:$false -ErrorAction SilentlyContinue
    Remove-NetRoute -InterfaceAlias $alias -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -Confirm:$false -ErrorAction SilentlyContinue
    if ($oldDhcp -eq 'Enabled') {
      Set-NetIPInterface -InterfaceAlias $alias -AddressFamily IPv4 -Dhcp Enabled -ErrorAction SilentlyContinue
      Set-DnsClientServerAddress -InterfaceAlias $alias -ResetServerAddresses -ErrorAction SilentlyContinue
    } elseif ($old) {
      Set-NetIPInterface -InterfaceAlias $alias -AddressFamily IPv4 -Dhcp Disabled -ErrorAction SilentlyContinue
      if ($oldGw) {
        New-NetIPAddress -InterfaceAlias $alias -IPAddress $old.IPAddress -PrefixLength $old.PrefixLength -DefaultGateway $oldGw -ErrorAction SilentlyContinue | Out-Null
      } else {
        New-NetIPAddress -InterfaceAlias $alias -IPAddress $old.IPAddress -PrefixLength $old.PrefixLength -ErrorAction SilentlyContinue | Out-Null
      }
      if ($oldDns.Count -gt 0) {
        Set-DnsClientServerAddress -InterfaceAlias $alias -ServerAddresses $oldDns -ErrorAction SilentlyContinue
      } else {
        Set-DnsClientServerAddress -InterfaceAlias $alias -ResetServerAddresses -ErrorAction SilentlyContinue
      }
    }
  } catch {}"#;

pub fn build_script(
    adapter: &str,
    use_static: bool,
    ip: &str,
    prefix: &str,
    gateway: &str,
    dns1: &str,
    dns2: &str,
) -> String {
    let adapter = ps_quote(adapter);

    // 이 조각은 자체적으로 format! 처리해서 어댑터 이름을 치환한다.
    let capture = format!(
        r#"$ErrorActionPreference='Stop'
$alias='{adapter}'
$old=Get-NetIPAddress -InterfaceAlias $alias -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -First 1
$oldGw=(Get-NetRoute -InterfaceAlias $alias -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Select-Object -First 1).NextHop
$oldDns=@((Get-DnsClientServerAddress -InterfaceAlias $alias -AddressFamily IPv4 -ErrorAction SilentlyContinue).ServerAddresses)
$oldDhcp=(Get-NetIPInterface -InterfaceAlias $alias -AddressFamily IPv4 -ErrorAction SilentlyContinue).Dhcp"#,
        adapter = adapter
    );

    let script = if use_static {
        let gateway_arg = if gateway.is_empty() {
            String::new()
        } else {
            format!(" -DefaultGateway '{}'", ps_quote(gateway))
        };
        let dns_items: Vec<String> = [dns1, dns2]
            .iter()
            .filter(|d| !d.is_empty())
            .map(|d| format!("'{}'", ps_quote(d)))
            .collect();
        let dns_setup = if dns_items.is_empty() {
            "Set-DnsClientServerAddress -InterfaceAlias $alias -ResetServerAddresses".to_string()
        } else {
            format!(
                "Set-DnsClientServerAddress -InterfaceAlias $alias -ServerAddresses @({})",
                dns_items.join(",")
            )
        };

        format!(
            r#"{capture}
try {{
  # DHCP가 켜진 인터페이스에 고정 주소를 만들면
  # "Inconsistent parameters PolicyStore PersistentStore and Dhcp Enabled" 오류가 발생한다.
  # 반드시 DHCP를 먼저 끄고 주소를 설정한다.
  Set-NetIPInterface -InterfaceAlias $alias -AddressFamily IPv4 -Dhcp Disabled -ErrorAction Stop
  Get-NetIPAddress -InterfaceAlias $alias -AddressFamily IPv4 -ErrorAction SilentlyContinue | Remove-NetIPAddress -Confirm:$false -ErrorAction SilentlyContinue
  Remove-NetRoute -InterfaceAlias $alias -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -Confirm:$false -ErrorAction SilentlyContinue
  New-NetIPAddress -InterfaceAlias $alias -IPAddress '{ip}' -PrefixLength {prefix}{gateway_arg} | Out-Null
  {dns_setup}
  'OK'
}} catch {{
  $err=$_.Exception.Message
{restore}
  [Console]::Error.WriteLine($err)
  exit 1
}}"#,
            capture = capture,
            ip = ps_quote(ip),
            prefix = prefix,
            gateway_arg = gateway_arg,
            dns_setup = dns_setup,
            restore = RESTORE_BLOCK,
        )
    } else {
        format!(
            r#"{capture}
try {{
  Get-NetIPAddress -InterfaceAlias $alias -AddressFamily IPv4 -ErrorAction SilentlyContinue | Remove-NetIPAddress -Confirm:$false -ErrorAction SilentlyContinue
  Remove-NetRoute -InterfaceAlias $alias -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -Confirm:$false -ErrorAction SilentlyContinue
  Set-NetIPInterface -InterfaceAlias $alias -AddressFamily IPv4 -Dhcp Enabled
  Set-DnsClientServerAddress -InterfaceAlias $alias -ResetServerAddresses
  'OK'
}} catch {{
  $err=$_.Exception.Message
{restore}
  [Console]::Error.WriteLine($err)
  exit 1
}}"#,
            capture = capture,
            restore = RESTORE_BLOCK,
        )
    };

    script
}

pub fn apply_settings(
    adapter: &str,
    use_static: bool,
    ip: &str,
    prefix: &str,
    gateway: &str,
    dns1: &str,
    dns2: &str,
) -> Result<String, String> {
    run_powershell(&build_script(
        adapter, use_static, ip, prefix, gateway, dns1, dns2,
    ))?;
    Ok(if use_static {
        "done_static".to_string()
    } else {
        "done_dhcp".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_validation() {
        assert!(is_valid_ipv4("192.168.0.10"));
        assert!(!is_valid_ipv4("192.168.0.010"));
        assert!(!is_valid_ipv4("256.1.1.1"));
        assert!(!is_valid_ipv4("1.2.3"));
        assert!(is_valid_ipv4(" 1.2.3.4 ")); // 앞뒤 공백은 trim 후 허용
        assert!(!is_valid_ipv4("1.2.3.4.5"));
    }

    #[test]
    fn static_validation() {
        assert_eq!(
            validate_static("192.168.0.10", "24", "192.168.0.1", "", ""),
            Ok(24)
        );
        assert_eq!(
            validate_static("192.168.0.10", "24", "192.168.1.1", "", ""),
            Err("status_invalid_gateway")
        );
        assert_eq!(
            validate_static("192.168.0.255", "24", "192.168.0.1", "", ""),
            Err("status_invalid_ip")
        );
        assert_eq!(
            validate_static("192.168.0.0", "24", "192.168.0.1", "", ""),
            Err("status_invalid_ip")
        );
        assert_eq!(
            validate_static("10.0.0.5", "33", "", "", ""),
            Err("status_invalid_prefix")
        );
        assert_eq!(
            validate_static("10.0.0.5", "24", "", "8.8.8.8", "bad"),
            Err("status_invalid_dns2")
        );
        assert_eq!(validate_static("10.0.0.5", "24", "", "8.8.8.8", ""), Ok(24));
    }

    #[test]
    fn ps_quoting() {
        assert_eq!(ps_quote("a'b"), "a''b");
    }

    #[test]
    fn statics_restore_block_has_single_braces() {
        // format!에 값으로 들어가므로 이스케이프된 중괄호가 없어야 한다.
        assert!(!RESTORE_BLOCK.contains("{{"));
    }

    #[test]
    fn static_apply_disables_dhcp_before_setting_address() {
        // DHCP가 켜진 상태에서 New-NetIPAddress를 호출하면
        // "Inconsistent parameters PolicyStore PersistentStore and Dhcp Enabled" 오류가 난다.
        let script = build_script(
            "Wi-Fi",
            true,
            "192.168.0.10",
            "24",
            "192.168.0.1",
            "8.8.8.8",
            "8.8.4.4",
        );
        let dhcp_off = script
            .find("-AddressFamily IPv4 -Dhcp Disabled")
            .expect("고정 IP 설정 시 DHCP를 끄는 명령이 없음");
        let new_ip = script
            .find("New-NetIPAddress")
            .expect("New-NetIPAddress 명령이 없음");
        assert!(
            dhcp_off < new_ip,
            "DHCP를 끄기 전에 고정 IP를 설정하고 있음"
        );
        // 주소 설정은 DHCP를 끈 뒤에만 나와야 한다.
        assert!(script.contains("Get-NetIPAddress"));
        assert!(script.contains("Set-DnsClientServerAddress"));
    }

    #[test]
    fn subnet_mask_to_prefix() {
        assert_eq!(mask_to_prefix("255.255.255.0"), Some(24));
        assert_eq!(mask_to_prefix("255.255.0.0"), Some(16));
        assert_eq!(mask_to_prefix("255.255.255.255"), Some(32));
        assert_eq!(mask_to_prefix("0.0.0.0"), Some(0));
        assert_eq!(mask_to_prefix("255.0.255.0"), None);
        assert_eq!(mask_to_prefix("garbage"), None);
    }

    #[test]
    fn prefix_to_mask_round_trip() {
        assert_eq!(prefix_to_mask(24), "255.255.255.0");
        assert_eq!(prefix_to_mask(16), "255.255.0.0");
        assert_eq!(prefix_to_mask(8), "255.0.0.0");
        assert_eq!(prefix_to_mask(32), "255.255.255.255");
        assert_eq!(prefix_to_mask(0), "0.0.0.0");
        for bits in 0..=32u8 {
            assert_eq!(mask_to_prefix(&prefix_to_mask(bits)), Some(bits));
        }
    }

    #[test]
    fn parses_subnet_forms() {
        assert_eq!(parse_subnet("255.255.255.0"), Some(24));
        assert_eq!(parse_subnet(" 255.255.0.0 "), Some(16));
        assert_eq!(parse_subnet("24"), Some(24));
        assert_eq!(parse_subnet("/24"), Some(24));
        assert_eq!(parse_subnet("33"), None);
        assert_eq!(parse_subnet("255.0.255.0"), None);
        assert_eq!(parse_subnet(""), None);
    }

    #[test]
    fn static_validation_accepts_dotted_mask() {
        assert_eq!(
            validate_static("192.168.0.10", "255.255.255.0", "192.168.0.1", "", ""),
            Ok(24)
        );
        assert_eq!(
            validate_static("192.168.0.10", "255.255.0.0", "192.168.0.1", "", ""),
            Ok(16)
        );
        assert_eq!(
            validate_static("192.168.0.10", "255.0.255.0", "192.168.0.1", "", ""),
            Err("status_invalid_prefix")
        );
    }

    #[test]
    fn parses_adapter_json() {
        let json = r#"[{"name":"Wi-Fi","description":"Intel AX211","wifi":true,"status":"up","ip":"192.168.0.5","mask":"255.255.255.0","gateway":"192.168.0.1","dns":["8.8.8.8"],"dhcp":true},
                       {"name":"이더넷","description":"Realtek GbE","wifi":false,"status":"disconnected","ip":null,"mask":null,"gateway":null,"dns":[],"dhcp":true}]"#;
        // Wi-Fi 전용: 유선 어댑터는 목록에서 빠진다.
        let adapters = parse_adapters(json);
        assert_eq!(adapters.len(), 1);
        assert!(adapters[0].wifi);
        assert_eq!(adapters[0].prefix(), Some(24));
        assert_eq!(adapters[0].ip.as_deref(), Some("192.168.0.5"));
        assert_eq!(adapters[0].dns, vec!["8.8.8.8".to_string()]);
        assert!(adapters[0].dhcp);
    }

    #[test]
    fn parses_single_object_and_garbage() {
        // PowerShell 5.1: 어댑터가 하나면 객체로, DNS가 하나면 문자열로 올 수 있다.
        let json = r#"{"name":"Wi-Fi","description":"d","wifi":true,"status":"up","ip":"1.2.3.4","mask":"255.255.255.0","gateway":"","dns":"1.1.1.1","dhcp":false}"#;
        let adapters = parse_adapters(json);
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].gateway, None);
        assert_eq!(adapters[0].dns, vec!["1.1.1.1".to_string()]);
        assert!(parse_adapters("").is_empty());
        assert!(parse_adapters("not json").is_empty());
        assert!(parse_adapters("[]").is_empty());
    }

    #[test]
    fn truncation_is_utf8_safe() {
        assert_eq!(truncate_chars("abc", 5), "abc");
        assert_eq!(truncate_chars("가나다라", 2), "가나…");
    }

    #[test]
    #[ignore]
    fn smoke_list_adapters() {
        let adapters = list_adapters().expect("adapter enumeration failed");
        assert!(!adapters.is_empty());
    }
}
