use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub description: String,
    pub services: Vec<String>,
    pub template: String,
    pub scaffold: Vec<String>,
    pub php_extensions: Vec<String>,
    pub env_template: Option<String>,
    pub dev_command: Option<String>,
}

pub fn get_blueprints() -> Vec<Blueprint> {
    vec![
        Blueprint {
            name: "laravel-vite".into(),
            description: "Laravel with Vite frontend bundler, MariaDB, and Redis".into(),
            services: vec!["nginx".into(), "php".into(), "mariadb".into(), "redis".into()],
            template: "laravel".into(),
            scaffold: vec!["composer create-project laravel/laravel .".into(), "npm install".into()],
            php_extensions: vec!["pdo_mysql".into(), "mbstring".into(), "openssl".into(), "tokenizer".into(), "xml".into(), "ctype".into(), "json".into(), "bcmath".into(), "redis".into()],
            env_template: Some("APP_NAME={{domain}}\nAPP_URL=http://{{domain}}\nDB_CONNECTION=mysql\nDB_HOST=127.0.0.1\nDB_PORT=3306\nDB_DATABASE={{db_name}}\nDB_USERNAME=root\nDB_PASSWORD=root\nCACHE_DRIVER=redis\nSESSION_DRIVER=redis\nREDIS_HOST=127.0.0.1\n".into()),
            dev_command: Some("npm run dev".into()),
        },
        Blueprint {
            name: "wordpress-woocommerce".into(),
            description: "WordPress with WooCommerce-ready configuration".into(),
            services: vec!["nginx".into(), "php".into(), "mariadb".into()],
            template: "wordpress".into(),
            scaffold: vec!["composer create-project johnpbloch/wordpress .".into()],
            php_extensions: vec!["pdo_mysql".into(), "gd".into(), "mbstring".into(), "xml".into(), "curl".into(), "zip".into(), "intl".into()],
            env_template: None,
            dev_command: None,
        },
        Blueprint {
            name: "nextjs-fullstack".into(),
            description: "Next.js full-stack application with nginx reverse proxy".into(),
            services: vec!["nginx".into(), "nodejs".into()],
            template: "nextjs".into(),
            scaffold: vec!["npx create-next-app@latest . --yes".into()],
            php_extensions: vec![],
            env_template: None,
            dev_command: Some("npm run dev".into()),
        },
        Blueprint {
            name: "astro-static".into(),
            description: "Astro static site generator".into(),
            services: vec!["nginx".into()],
            template: "astro".into(),
            scaffold: vec!["npm create astro@latest . -- --yes".into()],
            php_extensions: vec![],
            env_template: None,
            dev_command: Some("npm run dev".into()),
        },
        Blueprint {
            name: "django".into(),
            description: "Django web framework with nginx reverse proxy".into(),
            services: vec!["nginx".into(), "python".into()],
            template: "django".into(),
            scaffold: vec!["pip install django".into(), "django-admin startproject app .".into()],
            php_extensions: vec![],
            env_template: Some("DEBUG=True\nSECRET_KEY=change-me\nALLOWED_HOSTS={{domain}},localhost,127.0.0.1\nDATABASE_URL=sqlite:///db.sqlite3\n".into()),
            dev_command: Some("python manage.py runserver".into()),
        },
        Blueprint {
            name: "flask".into(),
            description: "Flask micro web framework with nginx reverse proxy".into(),
            services: vec!["nginx".into(), "python".into()],
            template: "django".into(),
            scaffold: vec!["pip install flask".into()],
            php_extensions: vec![],
            env_template: Some("FLASK_APP=app.py\nFLASK_ENV=development\nFLASK_DEBUG=1\n".into()),
            dev_command: Some("python -m flask run".into()),
        },
        Blueprint {
            name: "sveltekit".into(),
            description: "SvelteKit application with nginx reverse proxy".into(),
            services: vec!["nginx".into(), "nodejs".into()],
            template: "sveltekit".into(),
            scaffold: vec!["npm create svelte@latest . -- --yes".into()],
            php_extensions: vec![],
            env_template: None,
            dev_command: Some("npm run dev".into()),
        },
        Blueprint {
            name: "remix".into(),
            description: "Remix full-stack web framework".into(),
            services: vec!["nginx".into(), "nodejs".into()],
            template: "remix".into(),
            scaffold: vec!["npx create-remix@latest . --yes".into()],
            php_extensions: vec![],
            env_template: None,
            dev_command: Some("npm run dev".into()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_blueprints_count() {
        let blueprints = get_blueprints();
        assert_eq!(blueprints.len(), 8);
    }

    #[test]
    fn test_blueprints_basic_fields() {
        let blueprints = get_blueprints();
        for bp in blueprints {
            assert!(!bp.name.is_empty(), "Blueprint name should not be empty");
            assert!(!bp.description.is_empty(), "Blueprint description should not be empty");
            assert!(!bp.services.is_empty(), "Blueprint should have at least one service");
            assert!(!bp.template.is_empty(), "Blueprint should have a template");
        }
    }

    #[test]
    fn test_blueprint_templates() {
        let blueprints = get_blueprints();
        let valid_templates = vec![
            "laravel", "wordpress", "nextjs", "astro", 
            "django", "sveltekit", "remix"
        ];
        
        for bp in blueprints {
            assert!(
                valid_templates.contains(&bp.template.as_str()),
                "Invalid template '{}' in blueprint '{}'",
                bp.template,
                bp.name
            );
        }
    }

    #[test]
    fn test_php_extensions_presence() {
        let blueprints = get_blueprints();
        for bp in blueprints {
            let is_php = bp.services.contains(&"php".to_string());
            if is_php {
                assert!(!bp.php_extensions.is_empty(), "PHP blueprint {} must have extensions", bp.name);
            } else {
                assert!(bp.php_extensions.is_empty(), "Non-PHP blueprint {} should not have extensions", bp.name);
            }
        }
    }

    #[test]
    fn test_dev_commands() {
        let blueprints = get_blueprints();
        for bp in blueprints {
            if bp.name == "wordpress-woocommerce" {
                assert!(bp.dev_command.is_none(), "WordPress blueprint should not have dev_command");
            } else {
                assert!(bp.dev_command.is_some(), "Blueprint {} should have dev_command", bp.name);
                assert!(!bp.dev_command.as_ref().unwrap().is_empty());
            }
        }
    }

    #[test]
    fn test_blueprint_serde() {
        let blueprints = get_blueprints();
        let first = &blueprints[0];
        
        let json = serde_json::to_string(first).unwrap();
        let deserialized: Blueprint = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.name, first.name);
        assert_eq!(deserialized.services, first.services);
        assert_eq!(deserialized.php_extensions, first.php_extensions);
    }
}
