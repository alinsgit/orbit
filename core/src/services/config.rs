use std::path::PathBuf;
use std::fs;
use std::io::Write;

pub struct ConfigManager;

impl ConfigManager {
    pub fn ensure_nginx_config(nginx_root: &PathBuf) -> Result<(), String> {
        let conf_dir = nginx_root.join("conf");
        if !conf_dir.exists() {
            fs::create_dir_all(&conf_dir).map_err(|e| e.to_string())?;
        }

        let sites_enabled = conf_dir.join("sites-enabled");
        if !sites_enabled.exists() {
            fs::create_dir_all(&sites_enabled).map_err(|e| e.to_string())?;
        }

        let nginx_conf_path = conf_dir.join("nginx.conf");
        if !nginx_conf_path.exists() {
            let content = r#"
worker_processes  1;

events {
    worker_connections  1024;
}

http {
    include       mime.types;
    default_type  application/octet-stream;
    sendfile        on;
    keepalive_timeout  65;

    server {
        listen       80;
        server_name  localhost;

        location / {
            root   html;
            index  index.html index.htm;
        }

        error_page   500 502 503 504  /50x.html;
        location = /50x.html {
            root   html;
        }
    }

    include sites-enabled/*.conf;
}
"#;
            let mut file = fs::File::create(&nginx_conf_path).map_err(|e| e.to_string())?;
            file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
            
            // Also create mime.types if missing (simplified version)
            let mime_path = conf_dir.join("mime.types");
            if !mime_path.exists() {
                let mime_content = r#"
types {
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
    image/svg+xml                         svg svgz;
    image/tiff                            tif tiff;
    image/vnd.wap.wbmp                    wbmp;
    image/webp                            webp;
    image/x-icon                          ico;
    image/x-jng                           jng;
    image/x-ms-bmp                        bmp;

    application/font-woff                 woff;
    application/java-archive              jar war ear;
    application/json                      json;
    application/mac-binhex40              hqx;
    application/msword                    doc;
    application/pdf                       pdf;
    application/postscript                ps eps ai;
    application/rtf                       rtf;
    application/vnd.apple.mpegurl         m3u8;
    application/vnd.google-earth.kml+xml  kml;
    application/vnd.google-earth.kmz      kmz;
    application/vnd.ms-excel              xls;
    application/vnd.ms-fontobject         eot;
    application/vnd.ms-powerpoint         ppt;
    application/vnd.oasis.opendocument.graphics odg;
    application/vnd.oasis.opendocument.presentation odp;
    application/vnd.oasis.opendocument.spreadsheet ods;
    application/vnd.oasis.opendocument.text odt;
    application/vnd.openxmlformats-officedocument.presentationml.presentation pptx;
    application/vnd.openxmlformats-officedocument.spreadsheetml.sheet xlsx;
    application/vnd.openxmlformats-officedocument.wordprocessingml.document docx;
    application/vnd.wap.wmlc              wmlc;
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
    application/x-tcl                     tk pl;
    application/x-x509-ca-cert            der pem crt;
    application/x-xpinstall               xpi;
    application/xhtml+xml                 xhtml;
    application/xspf                      xspf;
    application/zip                       zip;

    application/octet-stream              bin exe dll;
    application/octet-stream              deb;
    application/octet-stream              dmg;
    application/octet-stream              iso img;
    application/octet-stream              msi msp msm;

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
                let mut mfile = fs::File::create(&mime_path).map_err(|e| e.to_string())?;
                mfile.write_all(mime_content.as_bytes()).map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn ensure_mariadb_config(mariadb_root: &PathBuf) -> Result<(), String> {
        let data_dir = mariadb_root.join("data");
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        }

        let conf_path = data_dir.join("my.ini");
        if !conf_path.exists() {
            // Use forward slashes for MySQL/MariaDB paths on Windows
            let data_path_str = data_dir.display().to_string().replace("\\", "/");

            let content = format!(
                r#"[mysqld]
datadir={}
port=3306
bind-address=127.0.0.1

[client]
port=3306
host=127.0.0.1

[mariadb]
"#,
                data_path_str
            );
            let mut file = fs::File::create(&conf_path).map_err(|e| e.to_string())?;
            file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn ensure_php_config(php_root: &PathBuf) -> Result<(), String> {
        let php_ini_path = php_root.join("php.ini");
        if !php_ini_path.exists() {
            // Check for php.ini-development
            let dev_ini = php_root.join("php.ini-development");
            if dev_ini.exists() {
                fs::copy(&dev_ini, &php_ini_path).map_err(|e| e.to_string())?;
            } else {
                // Minimal fallback
                let content = r#"
[PHP]
engine = On
short_open_tag = Off
precision = 14
output_buffering = 4096
zlib.output_compression = Off
implicit_flush = Off
serialize_precision = -1
zend.enable_gc = On
expose_php = On
max_execution_time = 30
max_input_time = 60
memory_limit = 128M
error_reporting = E_ALL
display_errors = On
display_startup_errors = On
log_errors = On
log_errors_max_len = 1024
ignore_repeated_errors = Off
ignore_repeated_source = Off
report_memleaks = On
html_errors = On
variables_order = "GPCS"
request_order = "GP"
register_argc_argv = Off
auto_globals_jit = On
post_max_size = 8M
auto_prepend_file =
auto_append_file =
default_mimetype = "text/html"
default_charset = "UTF-8"
doc_root =
user_dir =
enable_dl = Off
file_uploads = On
upload_max_filesize = 2M
max_file_uploads = 20
allow_url_fopen = On
allow_url_include = Off
default_socket_timeout = 60
extension_dir = "ext"
extension=curl
extension=fileinfo
extension=gd
extension=mbstring
extension=mysqli
extension=openssl
extension=pdo_mysql
extension=zip
"#;
                let mut file = fs::File::create(&php_ini_path).map_err(|e| e.to_string())?;
                file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub fn ensure_apache_config(apache_root: &PathBuf) -> Result<(), String> {
        let conf_dir = apache_root.join("conf");
        if !conf_dir.exists() {
            fs::create_dir_all(&conf_dir).map_err(|e| e.to_string())?;
        }

        let logs_dir = apache_root.join("logs");
        if !logs_dir.exists() {
            fs::create_dir_all(&logs_dir).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_ensure_nginx_config_creates_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        assert!(ConfigManager::ensure_nginx_config(&root).is_ok());
        
        assert!(root.join("conf").exists());
        assert!(root.join("conf/sites-enabled").exists());
        assert!(root.join("conf/nginx.conf").exists());
        assert!(root.join("conf/mime.types").exists());
    }

    #[test]
    fn test_ensure_nginx_config_idempotent() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        assert!(ConfigManager::ensure_nginx_config(&root).is_ok());
        assert!(ConfigManager::ensure_nginx_config(&root).is_ok()); // second call
        
        let conf_content = fs::read_to_string(root.join("conf/nginx.conf")).unwrap();
        assert!(conf_content.contains("worker_processes"));
    }

    #[test]
    fn test_ensure_mariadb_config_creates_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        assert!(ConfigManager::ensure_mariadb_config(&root).is_ok());
        
        assert!(root.join("data").exists());
        assert!(root.join("data/my.ini").exists());
    }

    #[test]
    fn test_ensure_mariadb_config_content() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        assert!(ConfigManager::ensure_mariadb_config(&root).is_ok());
        
        let content = fs::read_to_string(root.join("data/my.ini")).unwrap();
        assert!(content.contains("[mysqld]"));
        assert!(content.contains("port=3306"));
        // Forward slashes replace test
        assert!(!content.contains("\\"));
    }

    #[test]
    fn test_ensure_php_config_fallback() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        assert!(ConfigManager::ensure_php_config(&root).is_ok());
        
        let ini_path = root.join("php.ini");
        assert!(ini_path.exists());
        let content = fs::read_to_string(ini_path).unwrap();
        assert!(content.contains("[PHP]"));
        assert!(content.contains("extension=pdo_mysql"));
    }

    #[test]
    fn test_ensure_php_config_copies_dev() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        let dev_ini = root.join("php.ini-development");
        fs::write(&dev_ini, "custom development config").unwrap();
        
        assert!(ConfigManager::ensure_php_config(&root).is_ok());
        
        let ini_path = root.join("php.ini");
        let content = fs::read_to_string(ini_path).unwrap();
        assert_eq!(content, "custom development config");
    }

    #[test]
    fn test_ensure_apache_config_creates_dirs() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        
        assert!(ConfigManager::ensure_apache_config(&root).is_ok());
        
        assert!(root.join("conf").exists());
        assert!(root.join("logs").exists());
    }
}
