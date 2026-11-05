use crate::{daemon, models::*, store};
use fs2::FileExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::Mutex,
    time::Duration,
};
use uuid::Uuid;

fn network(e: impl std::fmt::Display) -> Failure {
    Failure::new("CONNECTION_ERROR",e.to_string(),"Inspect tala status and daemon.log; retry with the same --key if a send may have been stored.")
}
fn unreachable(port: u16, reason: &str) -> Failure {
    Failure::new(
        "DAEMON_UNREACHABLE",
        format!("Cannot reach the Tala daemon on 127.0.0.1:{port}: {reason}"),
        "The daemon may still be running: sandbox or network restrictions can block loopback connections, and a paused daemon stops answering. Tala will not start a second daemon. Request permission for local network access or inspect tala status.",
    )
}
fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(network)
}
/// Result of probing the daemon recorded in daemon.json.
pub enum Liveness {
    Live,
    /// Nothing usable answers at the recorded port: safe to start a daemon.
    Stale,
    /// The daemon may be alive but cannot be reached: never start another.
    Unreachable(String),
}
fn io_kind(error: &reqwest::Error) -> Option<std::io::ErrorKind> {
    let mut source = std::error::Error::source(error);
    while let Some(e) = source {
        if let Some(io) = e.downcast_ref::<std::io::Error>() {
            return Some(io.kind());
        }
        source = e.source();
    }
    None
}
fn classify(error: &reqwest::Error) -> Liveness {
    match io_kind(error) {
        Some(std::io::ErrorKind::ConnectionRefused) => Liveness::Stale,
        _ if error.is_timeout() || error.is_connect() => Liveness::Unreachable(
            io_kind(error)
                .map(|k| k.to_string())
                .unwrap_or_else(|| "timed out".into()),
        ),
        // Connected, but the peer answered badly: not a Tala daemon.
        _ => Liveness::Stale,
    }
}
pub async fn probe(info: &DaemonInfo) -> Liveness {
    // Classify the connection first. Windows retries a SYN to a closed port
    // for about two seconds before reporting refusal, so connecting gets its
    // own, longer limit: refusal means stale, anything else unreachable.
    let connect = tokio::net::TcpStream::connect(("127.0.0.1", info.port));
    match tokio::time::timeout(Duration::from_secs(5), connect).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => return Liveness::Stale,
        Ok(Err(e)) => return Liveness::Unreachable(e.kind().to_string()),
        Err(_) => return Liveness::Unreachable("connect timed out".into()),
    }
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Liveness::Unreachable(e.to_string()),
    };
    let endpoint = if info.instance.is_empty() {
        "/api/status"
    } else {
        "/health"
    };
    let result = client
        .get(format!("http://127.0.0.1:{}{}", info.port, endpoint))
        .send()
        .await;
    match result {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v)
                if (info.instance.is_empty() && v["pid"].as_u64() == Some(info.pid as u64))
                    || (!info.instance.is_empty() && v["instance"] == info.instance) =>
            {
                Liveness::Live
            }
            Ok(_) => Liveness::Stale,
            Err(e) if e.is_timeout() => Liveness::Unreachable("timed out".into()),
            Err(_) => Liveness::Stale,
        },
        Ok(_) => Liveness::Stale,
        Err(e) => classify(&e),
    }
}
pub async fn live(info: &DaemonInfo) -> bool {
    matches!(probe(info).await, Liveness::Live)
}
/// Probes recorded metadata: Some(info) when live, None when stale or absent.
async fn usable() -> Result<Option<DaemonInfo>> {
    let Some(info) = daemon::read_info() else {
        return Ok(None);
    };
    match probe(&info).await {
        Liveness::Live => compatible(info).map(Some),
        Liveness::Stale => Ok(None),
        Liveness::Unreachable(reason) => Err(unreachable(info.port, &reason)),
    }
}
fn startup_lock_contended(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    #[cfg(windows)]
    {
        // LockFileEx reports lock/sharing contention with these Win32 codes.
        matches!(error.raw_os_error(), Some(32) | Some(33))
    }
    #[cfg(not(windows))]
    {
        false
    }
}
pub async fn ensure() -> Result<DaemonInfo> {
    if let Some(i) = usable().await? {
        return Ok(i);
    }
    let lock_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let lock = daemon::lock_file("startup.lock")?;
    loop {
        match lock.try_lock_exclusive() {
            Ok(()) => break,
            Err(error) if startup_lock_contended(&error) => {
                // Another client may be starting a replacement. Let it make
                // progress on this runtime thread, and reuse the daemon as
                // soon as its health check succeeds.
                if let Some(i) = usable().await? {
                    return Ok(i);
                }
                if tokio::time::Instant::now() >= lock_deadline {
                    return Err(Failure::new(
                        "DAEMON_START_FAILED",
                        "Timed out waiting for another client to start the daemon",
                        "Check tala status and TALA_HOME/daemon.log, then retry.",
                    )
                    .with_next("tala status"));
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
    if let Some(i) = usable().await? {
        let _ = lock.unlock();
        return Ok(i);
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let log = options.open(store::home().join("daemon.log"))?;
    // Windows inherits every inheritable handle, including the CLI's capture
    // pipes. A long-lived daemon must not keep those pipes open after CLI exit.
    #[cfg(windows)]
    clear_standard_handle_inheritance()?;
    let mut command = Command::new(std::env::current_exe()?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000008 | 0x00000200); // Detached, new process group.
    }
    command
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()?;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(i) = usable().await? {
            let _ = lock.unlock();
            return Ok(i);
        }
    }
    Err(Failure::new(
        "DAEMON_START_FAILED",
        "Daemon did not start within five seconds",
        "Read TALA_HOME/daemon.log for storage, lock or startup errors.",
    ))
}

fn compatible(i: DaemonInfo) -> Result<DaemonInfo> {
    if i.protocol_version != PROTOCOL_VERSION {
        return Err(Failure::new(
            "PROTOCOL_MISMATCH",
            format!(
                "Daemon protocol {} differs from CLI protocol {}",
                i.protocol_version, PROTOCOL_VERSION
            ),
            "Stop the old daemon with its matching binary, then retry.",
        ));
    }
    Ok(i)
}
fn handle_path(handle: &str) -> std::path::PathBuf {
    let digest = Sha256::digest(handle.as_bytes());
    let name: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    store::home().join("handles").join(format!("{name}.json"))
}
/// Issues a new private handle for a credential and stores it under the
/// handle's hash, so neither the handle nor the token appears in file names.
pub fn issue_handle(credential: &Credential) -> Result<String> {
    let handle = format!("th_{}", Uuid::new_v4().simple());
    let path = handle_path(&handle);
    store::private_dir(path.parent().expect("handles directory"))?;
    store::private_write(&path, &serde_json::to_vec(credential).map_err(network)?)?;
    Ok(handle)
}
/// Credential stored by a protocol 2 registration, keyed by public ID.
pub fn legacy_credential_path(agent: &str) -> std::path::PathBuf {
    store::home()
        .join("identities")
        .join(format!("{agent}.json"))
}
/// Keeps a committed registration recoverable if writing its normal handle
/// mapping fails. `register --reissue` consumes this private fallback.
pub fn save_recovery_credential(credential: &Credential) -> Result<()> {
    let path = legacy_credential_path(&credential.agent);
    store::private_dir(path.parent().expect("identity directory"))?;
    store::private_write(&path, &serde_json::to_vec(credential).map_err(network)?)?;
    Ok(())
}
pub fn credential(handle: Option<&str>) -> Result<Credential> {
    let handle = handle.ok_or_else(|| {
        Failure::new(
            "IDENTITY_REQUIRED",
            "No agent selected",
            "Run tala register; pass the returned handle with --agent=<handle> or TALA_AGENT.",
        )
    })?;
    if handle.starts_with("agt_") {
        return Err(Failure::new(
            "IDENTITY_ERROR",
            "--agent takes your private handle, not a public agent ID",
            "Public agt_ IDs are for addressing others. Use the th_ handle that tala register returned to you. For your own identity created before handles existed, run tala register --reissue=<your agt_ ID> once.",
        ));
    }
    if !handle.starts_with("th_")
        || handle.len() != 35
        || !handle[3..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(Failure::new(
            "IDENTITY_ERROR",
            "--agent requires a complete th_ handle",
            "Use the handle returned by tala register.",
        ));
    }
    match std::fs::read(handle_path(handle)) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(network),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Failure::new(
            "IDENTITY_ERROR",
            "Unknown handle for this Tala home",
            "Use the handle tala register returned in this TALA_HOME, or register a new identity.",
        )),
        Err(e) => Err(e.into()),
    }
}

const MAX_SELECTOR_BYTES: usize = 37;

fn selector_failure(message: impl Into<String>) -> Failure {
    Failure::new(
        "IDENTITY_ERROR",
        message,
        "Use a private selector file containing only your th_ handle, or select an identity with --agent.",
    )
}

fn validate_handle(handle: &str) -> Result<()> {
    if handle.starts_with("th_")
        && handle.len() == 35
        && handle[3..].chars().all(|c| c.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(selector_failure(
            "Selector file must contain one complete th_ handle",
        ))
    }
}

/// Read and validate a private one-line selector file without following a
/// symlink/reparse point or accepting a file readable by other users.
pub fn read_selector_file(path: &Path) -> Result<String> {
    let file = open_selector_read(path)?;
    let mut bytes = Vec::new();
    file.take((MAX_SELECTOR_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| selector_failure(format!("Cannot read selector file: {e}")))?;
    if bytes.len() > MAX_SELECTOR_BYTES {
        return Err(selector_failure("Selector file is too large"));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| selector_failure("Selector file is not valid UTF-8"))?;
    let handle = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    validate_handle(handle)?;
    Ok(handle.to_owned())
}

/// Save a selector without replacing an existing path or changing its parent.
pub fn save_selector_file(path: &Path, handle: &str) -> Result<()> {
    validate_handle(handle)?;
    write_selector(path, format!("{handle}\n").as_bytes()).map_err(|e| {
        Failure::new(
            "IO_ERROR",
            format!("Could not save selector file: {e}"),
            "Choose a new path in a private directory; Tala never overwrites selector files.",
        )
    })
}

#[cfg(unix)]
fn open_selector_read(path: &Path) -> Result<File> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| selector_failure(format!("Cannot open selector file safely: {e}")))?;
    let metadata = file
        .metadata()
        .map_err(|e| selector_failure(format!("Cannot inspect selector file: {e}")))?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(selector_failure(
            "Selector file must be a regular file owned by this user with no group or other permissions",
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn write_selector(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent_meta = std::fs::metadata(parent)?;
    if !parent_meta.is_dir()
        || parent_meta.uid() != unsafe { libc::geteuid() }
        || parent_meta.mode() & 0o022 != 0
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "parent directory must be owned by the current user and not group/world writable",
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn open_selector_read(path: &Path) -> Result<File> {
    windows_selector::open_read(path).map_err(|e| {
        selector_failure(format!(
            "Cannot open or validate selector file securely: {e}"
        ))
    })
}

#[cfg(windows)]
fn write_selector(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    windows_selector::write_new(path, bytes)
}

#[cfg(not(any(unix, windows)))]
fn open_selector_read(_path: &Path) -> Result<File> {
    Err(selector_failure(
        "Private selector files are unsupported on this platform",
    ))
}

#[cfg(not(any(unix, windows)))]
fn write_selector(_path: &Path, _bytes: &[u8]) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "private selector files are unsupported on this platform",
    ))
}

#[cfg(windows)]
mod windows_selector {
    use std::{
        ffi::c_void,
        fs::File,
        io,
        os::windows::{ffi::OsStrExt, io::FromRawHandle},
        path::Path,
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, GetLastError, LocalFree, GENERIC_READ, GENERIC_WRITE, HANDLE,
            INVALID_HANDLE_VALUE,
        },
        Security::Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW,
            GetSecurityInfo, SE_FILE_OBJECT,
        },
        Security::{
            AclSizeInformation, EqualSid, GetAce, GetAclInformation, GetSecurityDescriptorControl,
            TokenUser, ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION,
            OWNER_SECURITY_INFORMATION, SECURITY_ATTRIBUTES, SE_DACL_PROTECTED, TOKEN_QUERY,
            TOKEN_USER,
        },
        Storage::FileSystem::{
            CreateFileW, FileAttributeTagInfo, GetFileInformationByHandleEx, CREATE_NEW,
            FILE_ALL_ACCESS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ, OPEN_EXISTING,
        },
        System::SystemServices::{ACCESS_ALLOWED_ACE_TYPE, SECURITY_DESCRIPTOR_REVISION},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    fn last_error() -> io::Error {
        io::Error::from_raw_os_error(unsafe { GetLastError() } as i32)
    }

    fn wide_path(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    struct UserSid {
        token: HANDLE,
        bytes: Vec<u8>,
    }

    impl UserSid {
        fn new() -> io::Result<Self> {
            let mut token = ptr::null_mut();
            if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
                return Err(last_error());
            }
            let mut needed = 0;
            unsafe {
                windows_sys::Win32::Security::GetTokenInformation(
                    token,
                    TokenUser,
                    ptr::null_mut(),
                    0,
                    &mut needed,
                );
            }
            if needed == 0 {
                unsafe { CloseHandle(token) };
                return Err(last_error());
            }
            let mut bytes = vec![0u8; needed as usize];
            if unsafe {
                windows_sys::Win32::Security::GetTokenInformation(
                    token,
                    TokenUser,
                    bytes.as_mut_ptr().cast(),
                    needed,
                    &mut needed,
                )
            } == 0
            {
                unsafe { CloseHandle(token) };
                return Err(last_error());
            }
            Ok(Self { token, bytes })
        }

        fn sid(&self) -> windows_sys::Win32::Security::PSID {
            unsafe { (*(self.bytes.as_ptr() as *const TOKEN_USER)).User.Sid }
        }

        fn sddl(&self) -> io::Result<Vec<u16>> {
            let mut sid_text = ptr::null_mut();
            if unsafe {
                windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW(
                    self.sid(),
                    &mut sid_text,
                )
            } == 0
            {
                return Err(last_error());
            }
            let sid = unsafe {
                let mut len = 0;
                while *sid_text.add(len) != 0 {
                    len += 1;
                }
                let text = String::from_utf16_lossy(std::slice::from_raw_parts(sid_text, len));
                LocalFree(sid_text.cast());
                text
            };
            Ok(format!("O:{sid}D:P(A;;FA;;;{sid})(A;;FA;;;SY)")
                .encode_utf16()
                .chain(Some(0))
                .collect())
        }
    }

    impl Drop for UserSid {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.token) };
        }
    }

    fn security_descriptor(sid: &UserSid) -> io::Result<*mut c_void> {
        let sddl = sid.sddl()?;
        let mut descriptor = ptr::null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SECURITY_DESCRIPTOR_REVISION,
                &mut descriptor,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(last_error());
        }
        Ok(descriptor)
    }

    fn validate_acl(handle: HANDLE, sid: &UserSid) -> io::Result<()> {
        unsafe {
            let mut owner = ptr::null_mut();
            let mut dacl: *mut ACL = ptr::null_mut();
            let mut descriptor = ptr::null_mut();
            let status = GetSecurityInfo(
                handle,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                ptr::null_mut(),
                &mut dacl,
                ptr::null_mut(),
                &mut descriptor,
            );
            if status != 0 {
                return Err(io::Error::from_raw_os_error(status as i32));
            }
            let mut control = 0;
            let mut revision = 0;
            let control_ok = GetSecurityDescriptorControl(descriptor, &mut control, &mut revision)
                != 0
                && control & SE_DACL_PROTECTED != 0;
            let owner_ok = !owner.is_null() && EqualSid(owner, sid.sid()) != 0;
            let mut system_sid = ptr::null_mut();
            let system_sddl: Vec<u16> = "S-1-5-18".encode_utf16().chain(Some(0)).collect();
            let system_ok = ConvertStringSidToSidW(system_sddl.as_ptr(), &mut system_sid) != 0;
            let mut acl_info = ACL_SIZE_INFORMATION::default();
            let acl_ok = !dacl.is_null()
                && GetAclInformation(
                    dacl,
                    (&mut acl_info as *mut ACL_SIZE_INFORMATION).cast(),
                    std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                    AclSizeInformation,
                ) != 0
                && acl_info.AceCount == 2;
            let mut saw_user = false;
            let mut saw_system = false;
            if acl_ok && system_ok {
                for index in 0..acl_info.AceCount {
                    let mut raw_ace = ptr::null_mut();
                    if GetAce(dacl, index, &mut raw_ace) == 0 || raw_ace.is_null() {
                        saw_user = false;
                        saw_system = false;
                        break;
                    }
                    let ace = &*(raw_ace as *const ACCESS_ALLOWED_ACE);
                    if ace.Header.AceType != ACCESS_ALLOWED_ACE_TYPE as u8
                        || ace.Header.AceFlags != 0
                        || ace.Mask != FILE_ALL_ACCESS
                    {
                        saw_user = false;
                        saw_system = false;
                        break;
                    }
                    let ace_sid = (&ace.SidStart as *const u32).cast_mut().cast();
                    if EqualSid(ace_sid, sid.sid()) != 0 && !saw_user {
                        saw_user = true;
                    } else if EqualSid(ace_sid, system_sid) != 0 && !saw_system {
                        saw_system = true;
                    } else {
                        saw_user = false;
                        saw_system = false;
                        break;
                    }
                }
            }
            if !system_sid.is_null() {
                LocalFree(system_sid.cast());
            }
            LocalFree(descriptor.cast());
            if control_ok && owner_ok && saw_user && saw_system {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "selector file ACL is not protected for this user and SYSTEM only",
                ))
            }
        }
    }

    fn open(path: &Path, disposition: u32, write: bool) -> io::Result<File> {
        let sid = UserSid::new()?;
        let mut descriptor = ptr::null_mut();
        let mut attributes = SECURITY_ATTRIBUTES::default();
        if write {
            descriptor = security_descriptor(&sid)?;
            attributes.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
            attributes.lpSecurityDescriptor = descriptor;
            attributes.bInheritHandle = 0;
        }
        let path_w = wide_path(path);
        let handle = unsafe {
            CreateFileW(
                path_w.as_ptr(),
                if write {
                    GENERIC_WRITE | GENERIC_READ
                } else {
                    GENERIC_READ
                },
                if write { 0 } else { FILE_SHARE_READ },
                if write {
                    &attributes as *const SECURITY_ATTRIBUTES
                } else {
                    ptr::null()
                },
                disposition,
                FILE_ATTRIBUTE_NORMAL
                    | FILE_FLAG_OPEN_REPARSE_POINT
                    | if write { FILE_FLAG_WRITE_THROUGH } else { 0 },
                ptr::null_mut(),
            )
        };
        if !descriptor.is_null() {
            unsafe { LocalFree(descriptor.cast()) };
        }
        if handle == INVALID_HANDLE_VALUE {
            return Err(last_error());
        }
        let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
        if unsafe {
            GetFileInformationByHandleEx(
                handle,
                FileAttributeTagInfo,
                (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
                std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
            )
        } == 0
        {
            let error = last_error();
            unsafe { CloseHandle(handle) };
            return Err(error);
        }
        if info.FileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0 {
            unsafe { CloseHandle(handle) };
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "selector path is a reparse point or directory",
            ));
        }
        if let Err(error) = validate_acl(handle, &sid) {
            unsafe { CloseHandle(handle) };
            return Err(error);
        }
        Ok(unsafe { File::from_raw_handle(handle) })
    }

    pub fn open_read(path: &Path) -> io::Result<File> {
        open(path, OPEN_EXISTING, false)
    }

    pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write;
        let mut file = open(path, CREATE_NEW, true)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    }
}
static ATTENTION: Mutex<Option<Value>> = Mutex::new(None);
/// Attention summary from the most recent successful daemon response.
pub fn take_attention() -> Option<Value> {
    ATTENTION.lock().ok().and_then(|mut a| a.take())
}
pub async fn call(op: Operation, credential: Option<Credential>) -> Result<Value> {
    call_as(Uuid::new_v4().to_string(), op, credential).await
}
/// Calls with a caller-chosen request ID; repeating an ID replays the
/// daemon's recorded result for state-changing operations.
pub async fn call_as(
    request_id: String,
    op: Operation,
    credential: Option<Credential>,
) -> Result<Value> {
    let request = Rpc {
        protocol_version: PROTOCOL_VERSION,
        request_id,
        credential,
        operation: op,
    };
    let mut last = None;
    for _ in 0..3 {
        let info = ensure().await?;
        match http()?
            .post(format!("http://127.0.0.1:{}/rpc", info.port))
            .header("x-tala-instance", &info.instance)
            .json(&request)
            .send()
            .await
        {
            Ok(response) => match response.json::<Envelope>().await {
                Ok(e) => {
                    return if e.ok {
                        if let Ok(mut slot) = ATTENTION.lock() {
                            *slot = e.attention;
                        }
                        e.data.ok_or_else(|| network("Empty response"))
                    } else {
                        Err(e.error.unwrap_or_else(|| network("Missing error")))
                    }
                }
                Err(e) => last = Some(network(e)),
            },
            Err(e) => last = Some(network(e)),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(last.unwrap_or_else(|| network("Request failed")))
}
pub async fn existing(op: Operation) -> Result<Option<Value>> {
    // Stop is deliberately allowed against a compatible daemon only; no stale PID signalling.
    let Some(info) = usable().await? else {
        return Ok(None);
    };
    let request = Rpc {
        protocol_version: PROTOCOL_VERSION,
        request_id: Uuid::new_v4().to_string(),
        credential: None,
        operation: op,
    };
    let response = http()?
        .post(format!("http://127.0.0.1:{}/rpc", info.port))
        .header("x-tala-instance", &info.instance)
        .json(&request)
        .send()
        .await
        .map_err(network)?
        .json::<Envelope>()
        .await
        .map_err(network)?;
    if !response.ok {
        return Err(response.error.unwrap_or_else(|| network("Missing error")));
    }
    Ok(response.data)
}

#[cfg(test)]
mod tests {
    use super::startup_lock_contended;

    #[test]
    fn would_block_is_startup_lock_contention() {
        let error = std::io::Error::from(std::io::ErrorKind::WouldBlock);
        assert!(startup_lock_contended(&error));
    }

    #[cfg(windows)]
    #[test]
    fn windows_lock_contention_codes_are_retryable_but_other_errors_are_not() {
        for code in [32, 33] {
            assert!(startup_lock_contended(&std::io::Error::from_raw_os_error(
                code
            )));
        }
        assert!(!startup_lock_contended(&std::io::Error::from_raw_os_error(
            5
        )));
    }
}

#[cfg(windows)]
fn clear_standard_handle_inheritance() -> std::io::Result<()> {
    use std::{ffi::c_void, os::windows::io::AsRawHandle};
    #[link(name = "kernel32")]
    extern "system" {
        fn SetHandleInformation(handle: *mut c_void, mask: u32, flags: u32) -> i32;
    }
    for handle in [
        std::io::stdin().as_raw_handle(),
        std::io::stdout().as_raw_handle(),
        std::io::stderr().as_raw_handle(),
    ] {
        if !handle.is_null() && handle as isize != -1 {
            // These are borrowed process handles; clearing inheritance does not
            // close them or interfere with the CLI's own input/output.
            if unsafe { SetHandleInformation(handle, 1, 0) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
    }
    Ok(())
}
