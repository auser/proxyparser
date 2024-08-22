use std::{collections::HashMap, fmt::Display};

use log::debug;
use misc_conf::{apache::Apache, ast::Directive};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    pub virtual_hosts: Vec<VirtualHost>,
    meta: HashMap<String, String>,
}

impl ProxyConfig {
    pub fn add_virtual_host(&mut self, virtual_host: VirtualHost) {
        self.virtual_hosts.push(virtual_host);
    }

    pub fn to_json(&self) -> Vec<serde_json::Value> {
        self.virtual_hosts
            .clone()
            .into_iter()
            .map(|virtual_host| virtual_host.into())
            .collect()
    }
}

impl Display for ProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for virtual_host in &self.virtual_hosts {
            writeln!(f, "Host: {}", virtual_host.host)?;
        }
        Ok(())
    }
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

impl VirtualHost {
    pub fn builder() -> VirtualHostBuilder {
        VirtualHostBuilder::default()
    }
}

#[derive(Default)]
pub struct VirtualHostBuilder {
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

impl VirtualHostBuilder {
    pub fn host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub fn server_name(mut self, server_name: String) -> Self {
        self.server_name = server_name;
        self
    }

    pub fn server_aliases(mut self, server_aliases: Vec<String>) -> Self {
        self.server_aliases = server_aliases;
        self
    }

    pub fn document_root(mut self, document_root: String) -> Self {
        self.document_root = document_root;
        self
    }

    pub fn custom_log(mut self, custom_log: String) -> Self {
        self.custom_log = custom_log;
        self
    }

    pub fn locations(mut self, locations: Vec<Location>) -> Self {
        self.locations = locations;
        self
    }

    pub fn rewrite_engine(mut self, rewrite_engine: bool) -> Self {
        self.rewrite_engine = rewrite_engine;
        self
    }

    pub fn rewrite_rules(mut self, rewrite_rules: Vec<RewriteRule>) -> Self {
        self.rewrite_rules = rewrite_rules;
        self
    }

    pub fn ssl_config(mut self, ssl_config: SslConfig) -> Self {
        self.ssl_config = ssl_config;
        self
    }

    pub fn env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn headers(mut self, headers: HashMap<String, Vec<String>>) -> Self {
        self.headers = headers;
        self
    }

    pub fn listen(mut self, listen: Vec<String>) -> Self {
        self.listen = listen;
        self
    }

    pub fn log_level(mut self, log_level: String) -> Self {
        self.log_level = log_level;
        self
    }

    pub fn build(self) -> VirtualHost {
        VirtualHost {
            host: self.host,
            server_name: self.server_name,
            server_aliases: self.server_aliases,
            document_root: self.document_root,
            custom_log: self.custom_log,
            locations: self.locations,
            rewrite_engine: self.rewrite_engine,
            rewrite_rules: self.rewrite_rules,
            ssl_config: self.ssl_config,
            env: self.env,
            headers: self.headers,
            listen: self.listen,
            log_level: self.log_level,
        }
    }
}

impl From<VirtualHost> for serde_json::Value {
    fn from(virtual_host: VirtualHost) -> Self {
        serde_json::to_value(virtual_host.server_name).unwrap()
        // json!({
        //     "host": virtual_host.host,
        //     "server_name": virtual_host.server_name,
        //     "server_aliases": virtual_host.server_aliases,
        //     "document_root": virtual_host.document_root,
        //     "custom_log": virtual_host.custom_log,
        //     "locations": virtual_host.locations,
        // })
    }
}

impl From<&HashMap<String, String>> for VirtualHost {
    fn from(row_values: &HashMap<String, String>) -> Self {
        VirtualHostBuilder::default()
            .host(row_values.get("Host").unwrap().to_string())
            .server_name(row_values.get("Server Name").unwrap().to_string())
            .document_root(row_values.get("Document Root").unwrap().to_string())
            .build()
    }
}

impl VirtualHost {
    pub fn to_etcd_config(&self) -> String {
        let mut config = String::new();
        debug!("VirtualHost: {:#?}", self);
        if self.server_name.is_empty() {
            return config;
        }
        let name = self.server_name.clone();
        let name = name.replace("http://", "");
        let name = name.replace("https://", "");
        let dashed_str = name.replace(".", "-");
        config.push_str(&format!(
            "etcdctl put traefik/http/routers/{dashed_str}/rule \"Host(\\`{name}\\`)\"\n"
        ));
        config.push_str(&format!(
            "etcdctl put traefik/http/routers/{dashed_str}/tls \"true\"\n"
        ));
        config.push_str(&format!(
            "etcdctl put traefik/http/routers/{dashed_str}/entryPoints/0 websecure\n"
        ));
        // for (index, rewrite_rule) in self.rewrite_rules.iter().enumerate() {
        //     config.push_str(&format!(
        //         "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/servers/{index}/url \"{}\"\n",
        //         rewrite_rule.replacement
        //     ));
        // }
        // Add middleware
        config.push_str(&format!(
            "etcdctl put traefik/http/routers/{dashed_str}/middlewares/0 https-only\n",
        ));
        config.push_str(&format!(
            "etcdctl put traefik/http/routers/{dashed_str}/middlewares/1 follow-redirects\n",
        ));

        config.push_str(&format!(
            "etcdctl put traefik/http/routers/{dashed_str}/service \"{dashed_str}\"\n",
        ));
        let host = self.host.clone();
        let (host, port) = host.split_once(":").unwrap_or((&self.host, "80"));
        let mut url = format!("http://{}", host);
        match port {
            "80" => {
                config.push_str(&format!(
                    "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/servers/0/scheme \"http\"\n",
                ));

                // config.push_str(&format!(
                //     "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/servers/0/url \"http://{}\"\n",
                //     host,
                // ));
            }
            "443" => {
                config.push_str(&format!(
                    "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/servers/0/scheme \"https\"\n",
                ));
                url = format!("https://{}", host);
            }
            _ => {
                url = format!("http://{}", host);
            }
        }
        config.push_str(&format!(
            "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/servers/0/url \"{}\"\n",
            url,
        ));

        config.push_str(&format!(
            "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/servers/0/port {port}\n"
        ));

        config.push_str(&format!(
            "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/responseForwarding/flushInterval \"100ms\"\n"
        ));
        // Server transports
        let transport_name = format!("{dashed_str}-transport");
        // Configure the server transport
        config.push_str(&format!(
            "etcdctl put traefik/http/serversTransports/{transport_name}/insecureSkipVerify \"true\"\n",
        ));
        config.push_str(&format!(
            "etcdctl put traefik/http/serversTransports/{transport_name}/forwardingTimeouts/responseHeaderTimeout \"30s\"\n",
        ));
        config.push_str(&format!(
            "etcdctl put traefik/http/serversTransports/{transport_name}/forwardingTimeouts/idleConnTimeout \"30s\"\n",
        ));

        // Set the serverTransport
        config.push_str(&format!(
            "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/serversTransport \"{transport_name}\"\n",
        ));

        // config.push_str(&format!(
        //     "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/serversTransport/forwardingTimeouts/responseHeaderTimeout \"30s\"\n"
        // ));
        // config.push_str(&format!(
        //     "etcdctl put traefik/http/services/{dashed_str}/loadbalancer/serversTransport/insecureSkipVerify true\n"
        // ));

        config
    }
}

impl Display for VirtualHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
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
            other => {
                pc.meta.insert(
                    other.to_string(),
                    serde_json::to_string(&directive.args).unwrap(),
                );
            }
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
        virtual_host
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
