use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Template information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub is_custom: bool,
    pub path: Option<String>,
}

/// Template manager for user-customizable templates
pub struct TemplateManager;

impl TemplateManager {
    /// Get the templates directory path
    pub fn get_templates_dir(bin_path: &PathBuf) -> PathBuf {
        bin_path.parent().unwrap_or(bin_path).join("templates")
    }

    /// Ensure templates directory exists and has default templates
    pub fn ensure_templates(bin_path: &PathBuf) -> Result<(), String> {
        let templates_dir = Self::get_templates_dir(bin_path);

        if !templates_dir.exists() {
            fs::create_dir_all(&templates_dir)
                .map_err(|e| format!("Failed to create templates dir: {}", e))?;
        }

        // Create default templates if they don't exist
        let defaults = [
            ("http", TEMPLATE_HTTP),
            ("https", TEMPLATE_HTTPS),
            ("static", TEMPLATE_STATIC),
            ("laravel", TEMPLATE_LARAVEL),
            ("wordpress", TEMPLATE_WORDPRESS),
            ("litecart", TEMPLATE_LITECART),
            ("reverse-proxy", TEMPLATE_REVERSE_PROXY),
        ];

        for (name, content) in defaults {
            let path = templates_dir.join(format!("{}.conf", name));
            if !path.exists() {
                fs::write(&path, content)
                    .map_err(|e| format!("Failed to write template {}: {}", name, e))?;
            }
        }

        Ok(())
    }

    /// List all available templates
    pub fn list_templates(bin_path: &PathBuf) -> Result<Vec<TemplateInfo>, String> {
        Self::ensure_templates(bin_path)?;

        let templates_dir = Self::get_templates_dir(bin_path);
        let mut templates = Vec::new();

        let descriptions = HashMap::from([
            ("http", "Standard HTTP PHP site"),
            ("https", "HTTPS site with SSL"),
            ("static", "Static HTML site (no PHP)"),
            ("laravel", "Laravel framework site"),
            ("wordpress", "WordPress CMS site"),
            ("litecart", "LiteCart e-commerce site"),
            ("reverse-proxy", "Reverse proxy for JS frameworks"),
        ]);

        if let Ok(entries) = fs::read_dir(&templates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "conf").unwrap_or(false) {
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy().to_string();
                        let description = descriptions
                            .get(name.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "Custom template".to_string());

                        let is_default = descriptions.contains_key(name.as_str());

                        templates.push(TemplateInfo {
                            name: name.clone(),
                            description,
                            is_custom: !is_default,
                            path: Some(path.to_string_lossy().to_string()),
                        });
                    }
                }
            }
        }

        // Sort: default templates first, then custom ones
        templates.sort_by(|a, b| {
            match (a.is_custom, b.is_custom) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(templates)
    }

    /// Get template content by name
    pub fn get_template(bin_path: &PathBuf, name: &str) -> Result<String, String> {
        Self::ensure_templates(bin_path)?;

        let templates_dir = Self::get_templates_dir(bin_path);
        let path = templates_dir.join(format!("{}.conf", name));

        if path.exists() {
            fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read template: {}", e))
        } else {
            // Fall back to default template
            match name {
                "http" => Ok(TEMPLATE_HTTP.to_string()),
                "https" => Ok(TEMPLATE_HTTPS.to_string()),
                "static" => Ok(TEMPLATE_STATIC.to_string()),
                "laravel" => Ok(TEMPLATE_LARAVEL.to_string()),
                "wordpress" => Ok(TEMPLATE_WORDPRESS.to_string()),
                "litecart" => Ok(TEMPLATE_LITECART.to_string()),
                "reverse-proxy" => Ok(TEMPLATE_REVERSE_PROXY.to_string()),
                _ => Err(format!("Template not found: {}", name)),
            }
        }
    }

    /// Save custom template content
    pub fn save_template(bin_path: &PathBuf, name: &str, content: &str) -> Result<(), String> {
        Self::ensure_templates(bin_path)?;

        let templates_dir = Self::get_templates_dir(bin_path);
        let path = templates_dir.join(format!("{}.conf", name));

        fs::write(&path, content)
            .map_err(|e| format!("Failed to save template: {}", e))
    }

    /// Reset a template to its default content
    pub fn reset_template(bin_path: &PathBuf, name: &str) -> Result<(), String> {
        let default_content = match name {
            "http" => Ok(TEMPLATE_HTTP),
            "https" => Ok(TEMPLATE_HTTPS),
            "static" => Ok(TEMPLATE_STATIC),
            "laravel" => Ok(TEMPLATE_LARAVEL),
            "wordpress" => Ok(TEMPLATE_WORDPRESS),
            "litecart" => Ok(TEMPLATE_LITECART),
            "reverse-proxy" => Ok(TEMPLATE_REVERSE_PROXY),
            _ => Err(format!("No default template for: {}", name)),
        }?;

        Self::save_template(bin_path, name, default_content)
    }

    /// Delete a custom template
    pub fn delete_template(bin_path: &PathBuf, name: &str) -> Result<(), String> {
        let defaults = ["http", "https", "static", "laravel", "wordpress", "litecart", "reverse-proxy"];
        if defaults.contains(&name) {
            return Err("Cannot delete default templates".to_string());
        }

        let templates_dir = Self::get_templates_dir(bin_path);
        let path = templates_dir.join(format!("{}.conf", name));

        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete template: {}", e))
        } else {
            Err("Template not found".to_string())
        }
    }
}

/// Simple template engine for nginx configs
pub struct TemplateEngine;

impl TemplateEngine {
    /// Replace {{variable}} placeholders with values
    pub fn render(template: &str, vars: &HashMap<&str, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// HTTP-only nginx site template
pub const TEMPLATE_HTTP: &str = r#"server {
    listen       {{port}};
    server_name  {{domain}};
    root         "{{path}}";

    index  index.php index.html index.htm;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    # PHP-FPM Configuration
    location ~ \.php$ {
        fastcgi_pass   127.0.0.1:{{php_port}};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        include        fastcgi_params;

        # Timeouts
        fastcgi_connect_timeout 60s;
        fastcgi_send_timeout 60s;
        fastcgi_read_timeout 60s;
    }

    # Deny access to hidden files
    location ~ /\. {
        deny all;
    }

    # Static file caching
    location ~* \.(jpg|jpeg|png|gif|ico|css|js|woff|woff2|ttf|svg)$ {
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
}
"#;

/// HTTPS nginx site template with SSL
pub const TEMPLATE_HTTPS: &str = r#"# HTTP to HTTPS redirect
server {
    listen       {{port}};
    server_name  {{domain}};
    return 301   https://$server_name$request_uri;
}

# HTTPS server
server {
    listen       {{ssl_port}} ssl;
    http2        on;
    server_name  {{domain}};
    root         "{{path}}";

    index  index.php index.html index.htm;

    # SSL Configuration
    ssl_certificate      "{{ssl_cert}}";
    ssl_certificate_key  "{{ssl_key}}";
    ssl_protocols        TLSv1.2 TLSv1.3;
    ssl_ciphers          HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    ssl_session_cache    shared:SSL:10m;
    ssl_session_timeout  10m;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    # PHP-FPM Configuration
    location ~ \.php$ {
        fastcgi_pass   127.0.0.1:{{php_port}};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        fastcgi_param  HTTPS on;
        include        fastcgi_params;

        # Timeouts
        fastcgi_connect_timeout 60s;
        fastcgi_send_timeout 60s;
        fastcgi_read_timeout 60s;
    }

    # Deny access to hidden files
    location ~ /\. {
        deny all;
    }

    # Static file caching
    location ~* \.(jpg|jpeg|png|gif|ico|css|js|woff|woff2|ttf|svg)$ {
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
}
"#;

/// Static site template (no PHP)
pub const TEMPLATE_STATIC: &str = r#"server {
    listen       {{port}};
    server_name  {{domain}};
    root         "{{path}}";

    index  index.html index.htm;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    location / {
        try_files $uri $uri/ =404;
    }

    # Deny access to hidden files
    location ~ /\. {
        deny all;
    }

    # Static file caching
    location ~* \.(jpg|jpeg|png|gif|ico|css|js|woff|woff2|ttf|svg)$ {
        expires 30d;
        add_header Cache-Control "public, immutable";
    }

    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml;
}
"#;

/// Laravel-specific template
pub const TEMPLATE_LARAVEL: &str = r#"server {
    listen       {{port}};
    server_name  {{domain}};
    root         "{{path}}/public";

    index  index.php index.html index.htm;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    # PHP-FPM Configuration
    location ~ \.php$ {
        fastcgi_pass   127.0.0.1:{{php_port}};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        include        fastcgi_params;

        # Laravel needs longer timeouts for artisan commands
        fastcgi_connect_timeout 300s;
        fastcgi_send_timeout 300s;
        fastcgi_read_timeout 300s;
    }

    # Deny access to hidden files except .well-known
    location ~ /\.(?!well-known).* {
        deny all;
    }

    # Deny PHP execution in sensitive Laravel directories (allow static assets)
    location ~ ^/(storage|bootstrap/cache)/.*\.php$ {
        deny all;
    }

    # Static file caching
    location ~* \.(jpg|jpeg|png|gif|ico|css|js|woff|woff2|ttf|svg)$ {
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
}
"#;

/// LiteCart e-commerce template (based on official documentation)
pub const TEMPLATE_LITECART: &str = r#"server {
    listen       {{port}};
    server_name  {{domain}};
    root         "{{path}}";

    index  index.php index.html index.htm;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    # Application - LiteCart URL rewriting
    location / {
        rewrite ^/(cache|images)/ /storage$uri last;
        try_files $uri $uri/ /index.php$is_args$args;
    }

    # PHP-FPM Configuration
    location ~ \.php$ {
        try_files $uri =404;
        fastcgi_pass   127.0.0.1:{{php_port}};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        fastcgi_param  HTTP_MOD_REWRITE On;
        fastcgi_param  HTTPS $https if_not_empty;
        fastcgi_param  HTTP_SCHEME $scheme;
        include        fastcgi_params;

        # Timeouts for e-commerce operations
        fastcgi_connect_timeout 120s;
        fastcgi_send_timeout 120s;
        fastcgi_read_timeout 120s;
    }

    # Return 404 for hidden files and directories starting with .
    location ~ /\. {
        return 404;
    }

    # Return 404 for specific extensions
    location ~ \.(htaccess|htpasswd|inc\.php|log|sql|bak|backup|old|tmp|env|conf|config)$ {
        return 404;
    }

    # Deny PHP execution in sensitive directories (allow static assets)
    location ~ ^/(data|logs|vendor|vmods)/.*\.php$ {
        deny all;
    }

    # CORS header for loading font files
    location ~* \.(eot|ttf|otf|woff|woff2)$ {
        add_header Access-Control-Allow-Origin "*" always;
    }

    # Static content cache and compression
    location ~* \.(a?png|avif|bmp|css|eot|gif|ico|jpe?g|jp2|js|otf|pdf|svg|tiff?|ttf|webp|woff2?)$ {
        expires 2w;
        gzip_static on;
    }
}
"#;

/// LiteCart e-commerce template with SSL
pub const TEMPLATE_LITECART_SSL: &str = r#"# HTTP to HTTPS redirect
server {
    listen       {{port}};
    server_name  {{domain}};
    return 301   https://$server_name$request_uri;
}

# HTTPS server
server {
    listen       {{ssl_port}} ssl;
    http2        on;
    server_name  {{domain}};
    root         "{{path}}";

    index  index.php index.html index.htm;

    # SSL Configuration
    ssl_certificate      "{{ssl_cert}}";
    ssl_certificate_key  "{{ssl_key}}";
    ssl_protocols        TLSv1.2 TLSv1.3;
    ssl_ciphers          HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    ssl_session_cache    shared:SSL:10m;
    ssl_session_timeout  10m;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Application - LiteCart URL rewriting
    location / {
        rewrite ^/(cache|images)/ /storage$uri last;
        try_files $uri $uri/ /index.php$is_args$args;
    }

    # PHP-FPM Configuration
    location ~ \.php$ {
        try_files $uri =404;
        fastcgi_pass   127.0.0.1:{{php_port}};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        fastcgi_param  HTTP_MOD_REWRITE On;
        fastcgi_param  HTTPS on;
        fastcgi_param  HTTP_SCHEME https;
        include        fastcgi_params;

        # Timeouts for e-commerce operations
        fastcgi_connect_timeout 120s;
        fastcgi_send_timeout 120s;
        fastcgi_read_timeout 120s;
    }

    # Return 404 for hidden files and directories starting with .
    location ~ /\. {
        return 404;
    }

    # Return 404 for specific extensions
    location ~ \.(htaccess|htpasswd|inc\.php|log|sql|bak|backup|old|tmp|env|conf|config)$ {
        return 404;
    }

    # Deny PHP execution in sensitive directories (allow static assets)
    location ~ ^/(data|logs|vendor|vmods)/.*\.php$ {
        deny all;
    }

    # CORS header for loading font files
    location ~* \.(eot|ttf|otf|woff|woff2)$ {
        add_header Access-Control-Allow-Origin "*" always;
    }

    # Static content cache and compression
    location ~* \.(a?png|avif|bmp|css|eot|gif|ico|jpe?g|jp2|js|otf|pdf|svg|tiff?|ttf|webp|woff2?)$ {
        expires 2w;
        gzip_static on;
    }
}
"#;

/// WordPress-specific template
pub const TEMPLATE_WORDPRESS: &str = r#"server {
    listen       {{port}};
    server_name  {{domain}};
    root         "{{path}}";

    index  index.php index.html index.htm;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    # WordPress permalinks
    location / {
        try_files $uri $uri/ /index.php?$args;
    }

    # PHP-FPM Configuration
    location ~ \.php$ {
        fastcgi_pass   127.0.0.1:{{php_port}};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        include        fastcgi_params;

        fastcgi_connect_timeout 60s;
        fastcgi_send_timeout 60s;
        fastcgi_read_timeout 60s;
    }

    # Deny access to sensitive files
    location ~* /(?:uploads|files)/.*\.php$ {
        deny all;
    }

    location ~ /\.ht {
        deny all;
    }

    location = /wp-config.php {
        deny all;
    }

    # Static file caching
    location ~* \.(jpg|jpeg|png|gif|ico|css|js|woff|woff2|ttf|svg)$ {
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
}
"#;

/// Reverse proxy nginx template (for JS frameworks: Next.js, Astro, Nuxt, Vue)
pub const TEMPLATE_REVERSE_PROXY: &str = r#"server {
    listen       {{port}};
    server_name  {{domain}};

    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    location / {
        proxy_pass http://127.0.0.1:{{dev_port}};
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
}
"#;

/// Reverse proxy nginx template with SSL
pub const TEMPLATE_REVERSE_PROXY_SSL: &str = r#"# HTTP to HTTPS redirect
server {
    listen       {{port}};
    server_name  {{domain}};
    return 301   https://$server_name$request_uri;
}

# HTTPS server
server {
    listen       {{ssl_port}} ssl;
    http2        on;
    server_name  {{domain}};

    # SSL Configuration
    ssl_certificate      "{{ssl_cert}}";
    ssl_certificate_key  "{{ssl_key}}";
    ssl_protocols        TLSv1.2 TLSv1.3;
    ssl_ciphers          HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    ssl_session_cache    shared:SSL:10m;
    ssl_session_timeout  10m;

    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    location / {
        proxy_pass http://127.0.0.1:{{dev_port}};
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
}
"#;

/// Apache reverse proxy vhost template (for JS frameworks)
pub const APACHE_TEMPLATE_REVERSE_PROXY: &str = r#"<VirtualHost *:{{port}}>
    ServerName {{domain}}

    ProxyPreserveHost On
    ProxyPass / http://127.0.0.1:{{dev_port}}/
    ProxyPassReverse / http://127.0.0.1:{{dev_port}}/

    # WebSocket support
    RewriteEngine On
    RewriteCond %{HTTP:Upgrade} websocket [NC]
    RewriteCond %{HTTP:Connection} upgrade [NC]
    RewriteRule /(.*) ws://127.0.0.1:{{dev_port}}/$1 [P,L]

    ErrorLog "logs/{{domain}}-error.log"
    CustomLog "logs/{{domain}}-access.log" combined
</VirtualHost>
"#;

/// Apache HTTP vhost template
pub const APACHE_TEMPLATE_HTTP: &str = r#"<VirtualHost *:{{port}}>
    ServerName {{domain}}
    DocumentRoot "{{path}}"

    <Directory "{{path}}">
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>

    # PHP-FPM via proxy
    <FilesMatch \.php$>
        SetHandler "proxy:fcgi://127.0.0.1:{{php_port}}"
    </FilesMatch>

    # Logs
    ErrorLog "logs/{{domain}}-error.log"
    CustomLog "logs/{{domain}}-access.log" combined
</VirtualHost>
"#;

/// Apache Laravel vhost template
pub const APACHE_TEMPLATE_LARAVEL: &str = r#"<VirtualHost *:{{port}}>
    ServerName {{domain}}
    DocumentRoot "{{path}}/public"

    <Directory "{{path}}/public">
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>

    # PHP-FPM via proxy
    <FilesMatch \.php$>
        SetHandler "proxy:fcgi://127.0.0.1:{{php_port}}"
    </FilesMatch>

    # Deny access to sensitive directories
    <DirectoryMatch "{{path}}/(storage|bootstrap/cache)">
        Require all denied
    </DirectoryMatch>

    # Logs
    ErrorLog "logs/{{domain}}-error.log"
    CustomLog "logs/{{domain}}-access.log" combined
</VirtualHost>
"#;

/// Apache WordPress vhost template
pub const APACHE_TEMPLATE_WORDPRESS: &str = r#"<VirtualHost *:{{port}}>
    ServerName {{domain}}
    DocumentRoot "{{path}}"

    <Directory "{{path}}">
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>

    # PHP-FPM via proxy
    <FilesMatch \.php$>
        SetHandler "proxy:fcgi://127.0.0.1:{{php_port}}"
    </FilesMatch>

    # Deny access to wp-config.php
    <Files wp-config.php>
        Require all denied
    </Files>

    # Deny PHP in uploads
    <Directory "{{path}}/wp-content/uploads">
        <FilesMatch \.php$>
            Require all denied
        </FilesMatch>
    </Directory>

    # Logs
    ErrorLog "logs/{{domain}}-error.log"
    CustomLog "logs/{{domain}}-access.log" combined
</VirtualHost>
"#;

/// Apache Static vhost template (no PHP)
pub const APACHE_TEMPLATE_STATIC: &str = r#"<VirtualHost *:{{port}}>
    ServerName {{domain}}
    DocumentRoot "{{path}}"

    <Directory "{{path}}">
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>

    # Logs
    ErrorLog "logs/{{domain}}-error.log"
    CustomLog "logs/{{domain}}-access.log" combined

    # Enable compression
    <IfModule mod_deflate.c>
        AddOutputFilterByType DEFLATE text/html text/plain text/css application/javascript
    </IfModule>

    # Cache static files
    <IfModule mod_expires.c>
        ExpiresActive On
        ExpiresByType image/jpg "access plus 1 week"
        ExpiresByType image/jpeg "access plus 1 week"
        ExpiresByType image/png "access plus 1 week"
        ExpiresByType image/gif "access plus 1 week"
        ExpiresByType text/css "access plus 1 week"
        ExpiresByType application/javascript "access plus 1 week"
    </IfModule>
</VirtualHost>
"#;

/// Apache LiteCart vhost template
pub const APACHE_TEMPLATE_LITECART: &str = r#"<VirtualHost *:{{port}}>
    ServerName {{domain}}
    DocumentRoot "{{path}}"

    <Directory "{{path}}">
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>

    # PHP-FPM via proxy
    <FilesMatch \.php$>
        SetHandler "proxy:fcgi://127.0.0.1:{{php_port}}"
    </FilesMatch>

    # Deny access to sensitive directories
    <DirectoryMatch "{{path}}/(data|logs|vendor|vmods)">
        <FilesMatch \.php$>
            Require all denied
        </FilesMatch>
    </DirectoryMatch>

    # Logs
    ErrorLog "logs/{{domain}}-error.log"
    CustomLog "logs/{{domain}}-access.log" combined
</VirtualHost>
"#;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SiteTemplate {
    Http,
    Https,
    Static,
    Laravel,
    WordPress,
    LiteCart,
    ReverseProxy,
}

impl SiteTemplate {
    /// Get Nginx template (default)
    #[allow(dead_code)]
    pub fn get_template(&self) -> &'static str {
        self.get_nginx_template()
    }

    /// Get Nginx template
    pub fn get_nginx_template(&self) -> &'static str {
        match self {
            SiteTemplate::Http => TEMPLATE_HTTP,
            SiteTemplate::Https => TEMPLATE_HTTPS,
            SiteTemplate::Static => TEMPLATE_STATIC,
            SiteTemplate::Laravel => TEMPLATE_LARAVEL,
            SiteTemplate::WordPress => TEMPLATE_WORDPRESS,
            SiteTemplate::LiteCart => TEMPLATE_LITECART,
            SiteTemplate::ReverseProxy => TEMPLATE_REVERSE_PROXY,
        }
    }

    /// Get Apache template
    pub fn get_apache_template(&self) -> &'static str {
        match self {
            SiteTemplate::Http | SiteTemplate::Https => APACHE_TEMPLATE_HTTP,
            SiteTemplate::Static => APACHE_TEMPLATE_STATIC,
            SiteTemplate::Laravel => APACHE_TEMPLATE_LARAVEL,
            SiteTemplate::WordPress => APACHE_TEMPLATE_WORDPRESS,
            SiteTemplate::LiteCart => APACHE_TEMPLATE_LITECART,
            SiteTemplate::ReverseProxy => APACHE_TEMPLATE_REVERSE_PROXY,
        }
    }

    pub fn detect_from_path(path: &str) -> Self {
        let path = std::path::Path::new(path);

        // Laravel detection
        if path.join("artisan").exists() && path.join("public").join("index.php").exists() {
            return SiteTemplate::Laravel;
        }

        // WordPress detection
        if path.join("wp-config.php").exists() || path.join("wp-config-sample.php").exists() {
            return SiteTemplate::WordPress;
        }

        // LiteCart detection - check for LiteCart-specific structure
        if path.join("includes").join("app_header.inc.php").exists()
            || (path.join("backend").exists()
                && path.join("frontend").exists()
                && path.join("includes").exists())
        {
            return SiteTemplate::LiteCart;
        }

        // Check for any PHP files
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|e| e == "php").unwrap_or(false) {
                    return SiteTemplate::Http;
                }
            }
        }

        // Default to static
        SiteTemplate::Static
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_render() {
        let mut vars = HashMap::new();
        vars.insert("domain", "test.local".to_string());
        vars.insert("port", "8080".to_string());
        vars.insert("path", "C:/projects/test".to_string());
        vars.insert("php_port", "9001".to_string());

        let result = TemplateEngine::render(TEMPLATE_HTTP, &vars);

        assert!(result.contains("server_name  test.local;"));
        assert!(result.contains("listen       8080;"));
        assert!(result.contains("fastcgi_pass   127.0.0.1:9001;"));
    }
}
