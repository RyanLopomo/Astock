//! Defesa dos arquivos locais e das conexões; não fornece criptografia.
use crate::{CoreError, Result};
use rusqlite::{config::DbConfig, limits::Limit, Connection};
use std::{
    fs,
    path::{Component, Path},
};

fn unsafe_path() -> CoreError {
    CoreError::new(
        "UNSAFE_PATH",
        "Use um diretório local, sem links, junções ou arquivos compartilhados por hard link.",
    )
}

/// Chamadores nativos escolhem caminhos fixos. Nenhum caminho vem do IPC.
pub fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(unsafe_path());
    }
    #[cfg(windows)]
    for component in path.components() {
        if let Component::Normal(name) = component {
            let name = name.to_string_lossy();
            let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
            let reserved = matches!(
                stem.as_str(),
                "CON"
                    | "PRN"
                    | "AUX"
                    | "NUL"
                    | "COM1"
                    | "COM2"
                    | "COM3"
                    | "COM4"
                    | "COM5"
                    | "COM6"
                    | "COM7"
                    | "COM8"
                    | "COM9"
                    | "LPT1"
                    | "LPT2"
                    | "LPT3"
                    | "LPT4"
                    | "LPT5"
                    | "LPT6"
                    | "LPT7"
                    | "LPT8"
                    | "LPT9"
            );
            if reserved || name.ends_with(['.', ' ']) || name.contains([':', '\0']) {
                return Err(unsafe_path());
            }
        }
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
    {
        return Err(unsafe_path());
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(unsafe_path());
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 {
                        return Err(unsafe_path());
                    }
                    if metadata.is_file() && windows::link_count(ancestor)? > 1 {
                        return Err(unsafe_path());
                    }
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.is_file() && metadata.nlink() > 1 {
                        return Err(unsafe_path());
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Restringe somente a pasta dedicada da aplicação e arquivos explicitamente conhecidos.
pub fn private_directory(path: &Path) -> Result<()> {
    validate_path(path)?;
    fs::create_dir_all(path)?;
    validate_path(path)?;
    private_file(path)
}

pub fn private_file(path: &Path) -> Result<()> {
    validate_path(path)?;
    #[cfg(windows)]
    windows::protect(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if path.is_dir() { 0o700 } else { 0o600 }),
        )?;
    }
    Ok(())
}

pub fn protect_database_files(path: &Path) -> Result<()> {
    validate_path(path)?;
    let parent = path.parent().ok_or_else(unsafe_path)?;
    for suffix in ["", "-wal", "-shm", "-journal"] {
        validate_path(Path::new(&format!("{}{suffix}", path.display())))?;
    }
    private_directory(parent)?;
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let file = std::path::PathBuf::from(format!("{}{suffix}", path.display()));
        if file.exists() {
            private_file(&file)?;
        }
    }
    let backups = parent.join("backups");
    if backups.exists() {
        private_directory(&backups)?;
        for entry in fs::read_dir(backups)? {
            let entry = entry?;
            validate_path(&entry.path())?;
            if entry.file_type()?.is_file() {
                private_file(&entry.path())?;
            }
        }
    }
    Ok(())
}

pub fn configure_connection(c: &Connection) -> Result<()> {
    c.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
    c.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    c.pragma_update(None, "temp_store", "MEMORY")?;
    c.set_limit(Limit::SQLITE_LIMIT_LENGTH, 16 * 1024 * 1024)?;
    c.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 256 * 1024)?;
    c.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;
    c.set_limit(Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 256)?;
    c.set_limit(Limit::SQLITE_LIMIT_EXPR_DEPTH, 100)?;
    c.set_limit(Limit::SQLITE_LIMIT_TRIGGER_DEPTH, 32)?;
    Ok(())
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        ffi::c_void,
        mem::{size_of, zeroed},
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree, HANDLE},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            },
            GetTokenInformation, SetFileSecurityW, TokenUser, DACL_SECURITY_INFORMATION,
            PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    struct Local(*mut c_void);
    impl Drop for Local {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
    fn win_error() -> CoreError {
        CoreError::new("FILE_PERMISSIONS", "Não foi possível proteger os arquivos locais. Verifique as permissões da pasta do usuário.")
    }
    pub(super) fn link_count(path: &Path) -> Result<u32> {
        let file = fs::File::open(path)?;
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
        // SAFETY: o handle está vivo e info é uma estrutura gravável do tamanho correto.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), &mut info) } == 0 {
            return Err(win_error());
        }
        Ok(info.nNumberOfLinks)
    }
    fn current_sid() -> Result<String> {
        let mut token = ptr::null_mut();
        // SAFETY: parâmetros de saída válidos; handle é fechado por RAII.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(win_error());
        }
        let token = Handle(token);
        let mut bytes = 0;
        unsafe {
            GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut bytes);
        }
        if bytes < size_of::<TOKEN_USER>() as u32 {
            return Err(win_error());
        }
        // usize fornece alinhamento suficiente para TOKEN_USER e seus ponteiros.
        let mut buffer = vec![0usize; (bytes as usize).div_ceil(size_of::<usize>())];
        if unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                bytes,
                &mut bytes,
            )
        } == 0
        {
            return Err(win_error());
        }
        let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        let mut sid_text = ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut sid_text) } == 0 {
            return Err(win_error());
        }
        let _allocated = Local(sid_text.cast());
        let mut length = 0;
        // SID convertido pelo Windows é terminado em NUL e permanece vivo até o retorno.
        while unsafe { *sid_text.add(length) } != 0 {
            length += 1;
        }
        String::from_utf16(unsafe { std::slice::from_raw_parts(sid_text, length) })
            .map_err(|_| win_error())
    }
    pub(super) fn protect(path: &Path) -> Result<()> {
        let sid = current_sid()?;
        let sddl: Vec<u16> = format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;{sid})")
            .encode_utf16()
            .chain([0])
            .collect();
        let mut descriptor = ptr::null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(win_error());
        }
        let _allocated = Local(descriptor);
        let filename: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        // DACL protegida: usuário atual, SYSTEM e Administradores. Nenhum dado/chave é registrado.
        if unsafe {
            SetFileSecurityW(
                filename.as_ptr(),
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                descriptor,
            )
        } == 0
        {
            return Err(win_error());
        }
        Ok(())
    }
}
