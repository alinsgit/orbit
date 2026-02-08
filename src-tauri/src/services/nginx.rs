use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};

pub struct NginxManager;

impl NginxManager {
    /// Get nginx binary path
    pub fn get_nginx_path(app: &AppHandle) -> Result<PathBuf, String> {
        let nginx_exe = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("nginx")
            .join("nginx.exe");

        if !nginx_exe.exists() {
            return Err("Nginx not installed".to_string());
        }

        Ok(nginx_exe)
    }

    /// Get nginx config directory
    pub fn get_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let config_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("nginx")
            .join("conf");

        Ok(config_dir)
    }

    /// Get sites-enabled directory
    pub fn get_sites_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let sites_dir = Self::get_config_dir(app)?.join("sites-enabled");

        if !sites_dir.exists() {
            std::fs::create_dir_all(&sites_dir)
                .map_err(|e| format!("Failed to create sites dir: {}", e))?;
        }

        Ok(sites_dir)
    }

    /// Get logs directory
    #[allow(dead_code)]
    pub fn get_logs_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let logs_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("bin")
            .join("nginx")
            .join("logs");

        if !logs_dir.exists() {
            std::fs::create_dir_all(&logs_dir)
                .map_err(|e| format!("Failed to create logs dir: {}", e))?;
        }

        Ok(logs_dir)
    }

    /// Test nginx configuration
    pub fn test_config(app: &AppHandle) -> Result<String, String> {
        let nginx_path = Self::get_nginx_path(app)?;
        let nginx_dir = nginx_path.parent().unwrap();

        let output = Command::new(&nginx_path)
            .current_dir(nginx_dir)
            .arg("-t")
            .output()
            .map_err(|e| format!("Failed to run nginx: {}", e))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() || stderr.contains("syntax is ok") {
            Ok("Configuration test successful".to_string())
        } else {
            Err(format!("Configuration error: {}", stderr))
        }
    }

    /// Reload nginx configuration
    pub fn reload(app: &AppHandle) -> Result<String, String> {
        // First test config
        Self::test_config(app)?;

        let nginx_path = Self::get_nginx_path(app)?;
        let nginx_dir = nginx_path.parent().unwrap();

        let output = Command::new(&nginx_path)
            .current_dir(nginx_dir)
            .args(["-s", "reload"])
            .output()
            .map_err(|e| format!("Failed to reload nginx: {}", e))?;

        if output.status.success() {
            Ok("Nginx reloaded successfully".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If nginx is not running, try to start it instead
            if stderr.contains("error") || stderr.contains("No such file") {
                return Err("Nginx is not running. Start it first.".to_string());
            }
            Err(format!("Reload failed: {}", stderr))
        }
    }

    /// Stop nginx
    #[allow(dead_code)]
    pub fn stop(app: &AppHandle) -> Result<String, String> {
        let nginx_path = Self::get_nginx_path(app)?;
        let nginx_dir = nginx_path.parent().unwrap();

        let output = Command::new(&nginx_path)
            .current_dir(nginx_dir)
            .args(["-s", "stop"])
            .output()
            .map_err(|e| format!("Failed to stop nginx: {}", e))?;

        if output.status.success() {
            Ok("Nginx stopped".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Stop failed: {}", stderr))
        }
    }

    /// Check if nginx is running
    pub fn is_running() -> bool {
        #[cfg(windows)]
        {
            let output = Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq nginx.exe"])
                .output();

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout.contains("nginx.exe")
                }
                Err(_) => false,
            }
        }

        #[cfg(not(windows))]
        {
            let output = Command::new("pgrep").arg("nginx").output();

            match output {
                Ok(out) => out.status.success(),
                Err(_) => false,
            }
        }
    }

    /// Ensure main nginx.conf includes sites-enabled
    pub fn ensure_main_config(app: &AppHandle) -> Result<(), String> {
        let config_dir = Self::get_config_dir(app)?;
        let main_conf = config_dir.join("nginx.conf");
        let sites_dir = Self::get_sites_dir(app)?;

        // Ensure sites-enabled directory exists
        if !sites_dir.exists() {
            std::fs::create_dir_all(&sites_dir)
                .map_err(|e| format!("Failed to create sites-enabled: {}", e))?;
        }

        // Check if main config exists
        if !main_conf.exists() {
            // Create a basic nginx.conf
            let content = Self::generate_main_config();
            std::fs::write(&main_conf, content)
                .map_err(|e| format!("Failed to write nginx.conf: {}", e))?;
        } else {
            // Check if sites-enabled is included
            let content = std::fs::read_to_string(&main_conf)
                .map_err(|e| format!("Failed to read nginx.conf: {}", e))?;

            if !content.contains("sites-enabled") {
                // Need to add include directive
                // Find the http block and add include
                let new_content = Self::add_sites_include(&content);
                std::fs::write(&main_conf, new_content)
                    .map_err(|e| format!("Failed to update nginx.conf: {}", e))?;
            }
        }

        // Ensure fastcgi_params exists
        let fastcgi_params = config_dir.join("fastcgi_params");
        if !fastcgi_params.exists() {
            std::fs::write(&fastcgi_params, FASTCGI_PARAMS)
                .map_err(|e| format!("Failed to write fastcgi_params: {}", e))?;
        }

        // Ensure mime.types exists
        let mime_types = config_dir.join("mime.types");
        if !mime_types.exists() {
            std::fs::write(&mime_types, MIME_TYPES)
                .map_err(|e| format!("Failed to write mime.types: {}", e))?;
        }

        Ok(())
    }

    fn generate_main_config() -> String {
        r#"worker_processes  1;

events {
    worker_connections  1024;
}

http {
    include       mime.types;
    default_type  application/octet-stream;

    sendfile        on;
    keepalive_timeout  65;

    # Gzip compression
    gzip  on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml;

    # Include site configurations
    include sites-enabled/*.conf;
}
"#
        .to_string()
    }

    fn add_sites_include(content: &str) -> String {
        // Simple approach: add before the last closing brace
        if let Some(pos) = content.rfind('}') {
            let mut new_content = content[..pos].to_string();
            new_content.push_str("\n    # Include site configurations\n");
            new_content.push_str("    include sites-enabled/*.conf;\n");
            new_content.push_str(&content[pos..]);
            new_content
        } else {
            content.to_string()
        }
    }
}

const FASTCGI_PARAMS: &str = r#"fastcgi_param  QUERY_STRING       $query_string;
fastcgi_param  REQUEST_METHOD     $request_method;
fastcgi_param  CONTENT_TYPE       $content_type;
fastcgi_param  CONTENT_LENGTH     $content_length;

fastcgi_param  SCRIPT_NAME        $fastcgi_script_name;
fastcgi_param  REQUEST_URI        $request_uri;
fastcgi_param  DOCUMENT_URI       $document_uri;
fastcgi_param  DOCUMENT_ROOT      $document_root;
fastcgi_param  SERVER_PROTOCOL    $server_protocol;
fastcgi_param  REQUEST_SCHEME     $scheme;
fastcgi_param  HTTPS              $https if_not_empty;

fastcgi_param  GATEWAY_INTERFACE  CGI/1.1;
fastcgi_param  SERVER_SOFTWARE    nginx/$nginx_version;

fastcgi_param  REMOTE_ADDR        $remote_addr;
fastcgi_param  REMOTE_PORT        $remote_port;
fastcgi_param  SERVER_ADDR        $server_addr;
fastcgi_param  SERVER_PORT        $server_port;
fastcgi_param  SERVER_NAME        $server_name;

# PHP only, required if PHP was built with --enable-force-cgi-redirect
fastcgi_param  REDIRECT_STATUS    200;
"#;

const MIME_TYPES: &str = r#"types {
    text/html                             html htm shtml;
    text/css                              css;
    text/xml                              xml;
    image/gif                             gif;
    image/jpeg                            jpeg jpg;
    application/javascript                js;
    application/atom+xml                  atom;
    application/rss+xml                   rss;

    text/mathml                           mml;
    text/plain                            txt;
    text/vnd.sun.j2me.app-descriptor      jad;
    text/vnd.wap.wml                      wml;
    text/x-component                      htc;

    image/png                             png;
    image/tiff                            tif tiff;
    image/vnd.wap.wbmp                    wbmp;
    image/x-icon                          ico;
    image/x-jng                           jng;
    image/x-ms-bmp                        bmp;
    image/svg+xml                         svg svgz;
    image/webp                            webp;

    application/font-woff                 woff;
    application/font-woff2                woff2;
    application/java-archive              jar war ear;
    application/json                      json;
    application/mac-binhex40              hqx;
    application/msword                    doc;
    application/pdf                       pdf;
    application/postscript                ps eps ai;
    application/rtf                       rtf;
    application/vnd.apple.mpegurl         m3u8;
    application/vnd.ms-excel              xls;
    application/vnd.ms-fontobject         eot;
    application/vnd.ms-powerpoint         ppt;
    application/vnd.wap.wmlc              wmlc;
    application/vnd.google-earth.kml+xml  kml;
    application/vnd.google-earth.kmz      kmz;
    application/x-7z-compressed           7z;
    application/x-cocoa                   cco;
    application/x-java-archive-diff       jardiff;
    application/x-java-jnlp-file          jnlp;
    application/x-makeself                run;
    application/x-perl                    pl pm;
    application/x-pilot                   prc pdb;
    application/x-rar-compressed          rar;
    application/x-redhat-package-manager  rpm;
    application/x-sea                     sea;
    application/x-shockwave-flash         swf;
    application/x-stuffit                 sit;
    application/x-tcl                     tcl tk;
    application/x-x509-ca-cert            der pem crt;
    application/x-xpinstall               xpi;
    application/xhtml+xml                 xhtml;
    application/xspf+xml                  xspf;
    application/zip                       zip;

    application/octet-stream              bin exe dll;
    application/octet-stream              deb;
    application/octet-stream              dmg;
    application/octet-stream              iso img;
    application/octet-stream              msi msp msm;

    application/vnd.openxmlformats-officedocument.wordprocessingml.document    docx;
    application/vnd.openxmlformats-officedocument.spreadsheetml.sheet          xlsx;
    application/vnd.openxmlformats-officedocument.presentationml.presentation  pptx;

    audio/midi                            mid midi kar;
    audio/mpeg                            mp3;
    audio/ogg                             ogg;
    audio/x-m4a                           m4a;
    audio/x-realaudio                     ra;

    video/3gpp                            3gpp 3gp;
    video/mp2t                            ts;
    video/mp4                             mp4;
    video/mpeg                            mpeg mpg;
    video/quicktime                       mov;
    video/webm                            webm;
    video/x-flv                           flv;
    video/x-m4v                           m4v;
    video/x-mng                           mng;
    video/x-ms-asf                        asx asf;
    video/x-ms-wmv                        wmv;
    video/x-msvideo                       avi;
}
"#;
