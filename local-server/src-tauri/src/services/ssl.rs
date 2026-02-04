use std::fs;
use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

const MKCERT_VERSION: &str = "v1.4.4";
const MKCERT_DOWNLOAD_URL: &str = "https://github.com/FiloSottile/mkcert/releases/download/v1.4.4/mkcert-v1.4.4-windows-amd64.exe";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslCertificate {
    pub domain: String,
    pub cert_path: String,
    pub key_path: String,
    pub created_at: String,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslStatus {
    pub mkcert_installed: bool,
    pub mkcert_path: String,
    pub ca_installed: bool,
    pub certificates: Vec<SslCertificate>,
}

pub struct SSLManager;

impl SSLManager {
    /// Get mkcert executable path
    pub fn get_mkcert_path(bin_path: &PathBuf) -> PathBuf {
        bin_path.join("mkcert").join("mkcert.exe")
    }

    /// Get SSL certificates directory
    pub fn get_certs_dir(bin_path: &PathBuf) -> PathBuf {
        bin_path.join("nginx").join("ssl")
    }

    /// Check if mkcert is installed
    pub fn is_mkcert_installed(bin_path: &PathBuf) -> bool {
        Self::get_mkcert_path(bin_path).exists()
    }

    /// Download and install mkcert
    pub async fn install_mkcert(bin_path: &PathBuf) -> Result<String, String> {
        let mkcert_dir = bin_path.join("mkcert");
        if !mkcert_dir.exists() {
            fs::create_dir_all(&mkcert_dir).map_err(|e| format!("Failed to create mkcert dir: {}", e))?;
        }

        let mkcert_path = Self::get_mkcert_path(bin_path);

        // Download mkcert
        let response = reqwest::get(MKCERT_DOWNLOAD_URL)
            .await
            .map_err(|e| format!("Failed to download mkcert: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to download mkcert: HTTP {}", response.status()));
        }

        let bytes = response.bytes().await.map_err(|e| format!("Failed to read mkcert: {}", e))?;
        fs::write(&mkcert_path, &bytes).map_err(|e| format!("Failed to save mkcert: {}", e))?;

        Ok(format!("mkcert {} installed successfully", MKCERT_VERSION))
    }

    /// Install mkcert root CA (requires admin)
    pub fn install_ca(bin_path: &PathBuf) -> Result<String, String> {
        let mkcert_path = Self::get_mkcert_path(bin_path);

        if !mkcert_path.exists() {
            return Err("mkcert is not installed".to_string());
        }

        let output = hidden_command(&mkcert_path)
            .arg("-install")
            .output()
            .map_err(|e| format!("Failed to run mkcert: {}", e))?;

        if output.status.success() {
            Ok("Root CA installed successfully. Browsers will now trust local certificates.".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Check if CA is already installed
            if stderr.contains("already exists") || stderr.contains("The local CA is already installed") {
                Ok("Root CA is already installed.".to_string())
            } else {
                Err(format!("Failed to install CA: {}", stderr))
            }
        }
    }

    /// Check if root CA is installed
    pub fn is_ca_installed(bin_path: &PathBuf) -> bool {
        let mkcert_path = Self::get_mkcert_path(bin_path);

        if !mkcert_path.exists() {
            return false;
        }

        // Run mkcert -CAROOT to check
        if let Ok(output) = hidden_command(&mkcert_path).arg("-CAROOT").output() {
            if output.status.success() {
                let ca_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let ca_cert = PathBuf::from(&ca_root).join("rootCA.pem");
                return ca_cert.exists();
            }
        }

        false
    }

    /// Generate SSL certificate for a domain using mkcert
    pub fn generate_cert(bin_path: &PathBuf, domain: &str) -> Result<SslCertificate, String> {
        let mkcert_path = Self::get_mkcert_path(bin_path);

        if !mkcert_path.exists() {
            return Err("mkcert is not installed. Please install it first.".to_string());
        }

        let certs_dir = Self::get_certs_dir(bin_path);
        if !certs_dir.exists() {
            fs::create_dir_all(&certs_dir).map_err(|e| format!("Failed to create ssl dir: {}", e))?;
        }

        let cert_path = certs_dir.join(format!("{}.pem", domain));
        let key_path = certs_dir.join(format!("{}-key.pem", domain));

        // Generate certificate
        let output = hidden_command(&mkcert_path)
            .current_dir(&certs_dir)
            .arg("-cert-file")
            .arg(&cert_path)
            .arg("-key-file")
            .arg(&key_path)
            .arg(domain)
            .arg(format!("*.{}", domain)) // Wildcard
            .arg("localhost")
            .arg("127.0.0.1")
            .arg("::1")
            .output()
            .map_err(|e| format!("Failed to run mkcert: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to generate certificate: {}", stderr));
        }

        let created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        Ok(SslCertificate {
            domain: domain.to_string(),
            cert_path: cert_path.to_string_lossy().to_string(),
            key_path: key_path.to_string_lossy().to_string(),
            created_at,
            is_valid: true,
        })
    }

    /// Get certificate for a domain (if exists)
    pub fn get_cert(bin_path: &PathBuf, domain: &str) -> Option<SslCertificate> {
        let certs_dir = Self::get_certs_dir(bin_path);
        let cert_path = certs_dir.join(format!("{}.pem", domain));
        let key_path = certs_dir.join(format!("{}-key.pem", domain));

        if cert_path.exists() && key_path.exists() {
            let modified = fs::metadata(&cert_path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default();

            Some(SslCertificate {
                domain: domain.to_string(),
                cert_path: cert_path.to_string_lossy().to_string(),
                key_path: key_path.to_string_lossy().to_string(),
                created_at: modified,
                is_valid: true,
            })
        } else {
            None
        }
    }

    /// List all certificates
    pub fn list_certs(bin_path: &PathBuf) -> Vec<SslCertificate> {
        let certs_dir = Self::get_certs_dir(bin_path);
        let mut certs = Vec::new();

        if let Ok(entries) = fs::read_dir(&certs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Only process .pem files that are not key files
                    if name.ends_with(".pem") && !name.ends_with("-key.pem") {
                        let domain = name.trim_end_matches(".pem");
                        if let Some(cert) = Self::get_cert(bin_path, domain) {
                            certs.push(cert);
                        }
                    }
                }
            }
        }

        certs
    }

    /// Delete certificate for a domain
    pub fn delete_cert(bin_path: &PathBuf, domain: &str) -> Result<(), String> {
        let certs_dir = Self::get_certs_dir(bin_path);
        let cert_path = certs_dir.join(format!("{}.pem", domain));
        let key_path = certs_dir.join(format!("{}-key.pem", domain));

        if cert_path.exists() {
            fs::remove_file(&cert_path).map_err(|e| format!("Failed to delete cert: {}", e))?;
        }
        if key_path.exists() {
            fs::remove_file(&key_path).map_err(|e| format!("Failed to delete key: {}", e))?;
        }

        Ok(())
    }

    /// Get full SSL status
    pub fn get_status(bin_path: &PathBuf) -> SslStatus {
        let mkcert_installed = Self::is_mkcert_installed(bin_path);
        let mkcert_path = Self::get_mkcert_path(bin_path).to_string_lossy().to_string();
        let ca_installed = if mkcert_installed { Self::is_ca_installed(bin_path) } else { false };
        let certificates = Self::list_certs(bin_path);

        SslStatus {
            mkcert_installed,
            mkcert_path,
            ca_installed,
            certificates,
        }
    }
}
