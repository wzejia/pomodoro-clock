//! 配置与统计共用的原子替换写。
//! 先把完整内容写入临时文件并 `sync_all`，再 `rename` 盖到目标路径，
//! 避免进程在写入中途退出时留下空文件或半截 JSON。

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// 把解析失败的原文件挪成带时间戳的 `.corrupt`。
/// 挪走之后，后续用空库 `save` 只会新建目标文件，不会盖掉原来的字节。
/// `rename` 失败时改为复制；复制也失败则返回错误，调用方不得覆盖原文件。
pub fn quarantine_corrupt(path: &Path) -> std::io::Result<PathBuf> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let original = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let mut dest = path.with_file_name(format!("{original}.{millis}.corrupt"));
    let mut n = 0u32;
    while dest.exists() {
        n += 1;
        dest = path.with_file_name(format!("{original}.{millis}-{n}.corrupt"));
        if n > 100 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "无法为损坏文件分配备份名",
            ));
        }
    }
    match std::fs::rename(path, &dest) {
        Ok(()) => Ok(dest),
        Err(_) => {
            std::fs::copy(path, &dest)?;
            // 字节已在备份里。删掉原路径，避免下一次 save 直接覆盖唯一副本。
            match std::fs::remove_file(path) {
                Ok(()) => Ok(dest),
                Err(e) => {
                    // 备份在，但原文件还在。告诉调用方失败，禁止覆盖。
                    let _ = std::fs::remove_file(&dest);
                    Err(e)
                }
            }
        }
    }
}

/// 测试辅助：制造「隔离必败」的现场，验证调用方不覆盖原文件。drop 时还原。
///
/// Unix：把父目录设为只读，rename/copy 建目标都 EACCES。
/// Windows：目录只读不阻止写入（实测 rename 照样成功）；且 Rust 1.96 std 的
/// File::open 默认 share 含 DELETE，持句柄也挡不住 rename——所以显式
/// share_mode(READ|WRITE) 排除 DELETE：rename/remove_file 都要 DELETE 权限 → 必败；
/// copy 只读源文件不受影响，会走到 remove_file 失败分支，照样返回 Err。
#[cfg(test)]
pub(crate) struct QuarantineGuard {
    #[cfg_attr(windows, allow(dead_code))]
    dir: PathBuf,
    #[cfg(windows)]
    _lock: std::fs::File,
}

#[cfg(test)]
pub(crate) fn block_quarantine(path: &Path) -> QuarantineGuard {
    let dir = path
        .parent()
        .expect("测试路径必须有父目录")
        .to_path_buf();
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&dir, perms).unwrap();
        QuarantineGuard { dir }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x1;
        const FILE_SHARE_WRITE: u32 = 0x2;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(path)
            .unwrap();
        QuarantineGuard { dir, _lock: lock }
    }
}

#[cfg(test)]
impl Drop for QuarantineGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&self.dir).unwrap().permissions();
            perms.set_readonly(false);
            std::fs::set_permissions(&self.dir, perms).unwrap();
        }
        // Windows：_lock 随 struct drop，句柄释放即还原。
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_replaces_target_and_removes_tmp() {
        let dir = std::env::temp_dir().join(format!("pomodoro-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.json");
        std::fs::write(&path, b"old").unwrap();
        write_atomic(&path, b"{\"version\":1}").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"version\":1}");
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
