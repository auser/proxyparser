use std::collections::HashMap;

use misc_conf::{apache::Apache, ast::Directive};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    virtual_hosts: Vec<VirtualHost>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VirtualHost {
    host: String,
    server_name: String,
    server_aliases: Vec<String>,
    document_root: String,
    custom_log: String,
    locations: Vec<Location>,
    rewrite_engine: bool,
    rewrite_rules: Vec<RewriteRule>,
    ssl_config: SslConfig,
    env: HashMap<String, String>,
    headers: HashMap<String, Vec<String>>,
    listen: Vec<String>,
    log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Location {
    path: String,
    allow_methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewriteRule {
    pattern: String,
    replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SslConfig {
    enabled: bool,
    certificate_file: String,
    key_file: String,
    chain_file: Option<String>,
    honor_cipher_order: bool,
    ciphers: Option<String>,
    protocols: Option<Vec<String>>,
}

impl From<Directive<Apache>> for ProxyConfig {
    fn from(directive: Directive<Apache>) -> Self {
        let mut pc = ProxyConfig::default();
        match directive.name.as_str() {
            "VirtualHost" => {
                let virtual_host = VirtualHost::from(directive);
                pc.virtual_hosts.push(virtual_host);
            }
            _ => {}
        };
        pc
    }
}

impl From<Directive<Apache>> for VirtualHost {
    fn from(directive: Directive<Apache>) -> Self {
        let mut virtual_host = VirtualHost::default();
        virtual_host.host = directive.args[0].to_string();
        virtual_host.ssl_config = SslConfig::default();
        directive.children.iter().for_each(|subchildren| {
            subchildren
                .into_iter()
                .for_each(|subchild| match subchild.name.as_str() {
                    "ServerName" => {
                        virtual_host.server_name = subchild.args[0].to_string();
                    }
                    "DocumentRoot" => {
                        virtual_host.document_root = subchild.args[0].to_string();
                    }
                    "ServerAlias" => {
                        virtual_host
                            .server_aliases
                            .push(subchild.args[0].to_string());
                    }
                    "Header" => {
                        virtual_host
                            .headers
                            .entry(subchild.args[0].to_string())
                            .and_modify(|x| x.push(subchild.args[1].to_string()))
                            .or_insert(vec![]);
                    }
                    "RequestHeader" => {
                        virtual_host
                            .headers
                            .entry(subchild.args[0].to_string())
                            .and_modify(|x| x.push(subchild.args[1].to_string()))
                            .or_insert(vec![]);
                    }
                    "RewriteEngine" => {
                        virtual_host.rewrite_engine = subchild.args[0].to_string() == "On";
                    }
                    "RewriteRule" | "ProxyPassReverse" | "ProxyPass" => {
                        virtual_host.rewrite_rules.push(RewriteRule::from(subchild));
                    }
                    "CustomLog" => {
                        virtual_host.custom_log = subchild.args[0].to_string();
                    }
                    "SSLEngine" => {
                        virtual_host.ssl_config.enabled =
                            subchild.args[0].to_string().to_lowercase() == "on";
                    }
                    "SSLCertificateFile" => {
                        virtual_host.ssl_config.certificate_file = subchild.args[0].to_string();
                    }
                    "SSLCertificateKeyFile" => {
                        virtual_host.ssl_config.key_file = subchild.args[0].to_string();
                    }
                    "SSLCertificateChainFile" => {
                        virtual_host.ssl_config.chain_file = Some(subchild.args[0].to_string());
                    }
                    "SSLHonorCipherOrder" => {
                        virtual_host.ssl_config.honor_cipher_order =
                            subchild.args[0].to_string().to_lowercase() == "on";
                    }
                    "SSLCiphers" => {
                        virtual_host.ssl_config.ciphers = Some(subchild.args[0].to_string());
                    }
                    "SSLProtocols" => {
                        virtual_host.ssl_config.protocols = Some(subchild.args.clone());
                    }
                    "LogLevel" => {
                        virtual_host.log_level = subchild.args[0].to_string();
                    }
                    _ => {}
                });
        });
        Self::default()
    }
}

impl From<&Directive<Apache>> for RewriteRule {
    fn from(directive: &Directive<Apache>) -> Self {
        let mut rewrite_rule = RewriteRule::default();
        rewrite_rule.pattern = directive.args[0].to_string();
        rewrite_rule.replacement = directive.args[1].to_string();
        rewrite_rule
    }
}
