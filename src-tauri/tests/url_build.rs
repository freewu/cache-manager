// build_url / build_fred_config 的单元测试（不需要真实 Redis）

use cache_manager_lib::model::{ConnConfig, ConnMode, NodeSpec};
use cache_manager_lib::redisx::build_fred_config;
use fred::prelude::*;

fn base() -> ConnConfig {
    ConnConfig {
        id: "t".into(),
        name: "t".into(),
        mode: ConnMode::Single,
        host: "127.0.0.1".into(),
        port: 6379,
        username: None,
        password: None,
        database: Some(2),
        nodes: Vec::new(),
        service_name: None,
        tls: false,
        connect_timeout_ms: 5000,
    }
}

#[test]
fn test_single_url() {
    let cfg = base();
    let fred_cfg = build_fred_config(&cfg).unwrap();
    match fred_cfg.server {
        ServerConfig::Centralized { ref server } => {
            assert_eq!(server.host, "127.0.0.1");
            assert_eq!(server.port, 6379);
        }
        other => panic!("期望 Centralized, got {:?}", other),
    }
    assert_eq!(fred_cfg.database, Some(2));
}

#[test]
fn test_single_with_auth_and_tls() {
    let mut cfg = base();
    cfg.username = Some("alice".into());
    cfg.password = Some("p@ss".into());
    cfg.tls = true;
    let fred_cfg = build_fred_config(&cfg).unwrap();
    assert_eq!(fred_cfg.username.as_deref(), Some("alice"));
    assert_eq!(fred_cfg.password.as_deref(), Some("p@ss"));
    assert!(fred_cfg.uses_tls());
}

#[test]
fn test_cluster_url() {
    let mut cfg = base();
    cfg.mode = ConnMode::Cluster;
    cfg.port = 7000;
    cfg.nodes = vec![
        NodeSpec { host: "10.0.0.2".into(), port: 7001 },
        NodeSpec { host: "10.0.0.3".into(), port: 7002 },
    ];
    let fred_cfg = build_fred_config(&cfg).unwrap();
    match fred_cfg.server {
        ServerConfig::Clustered { ref hosts, .. } => {
            let mut ports: Vec<u16> = hosts.iter().map(|h| h.port).collect();
            ports.sort();
            assert_eq!(ports, vec![7000, 7001, 7002]);
        }
        other => panic!("期望 Clustered, got {:?}", other),
    }
    // 集群不使用 db
    assert_eq!(fred_cfg.database, None);
}

#[test]
fn test_sentinel_url() {
    let mut cfg = base();
    cfg.mode = ConnMode::Sentinel;
    cfg.nodes = vec![NodeSpec { host: "10.0.0.2".into(), port: 26380 }];
    cfg.service_name = Some("mymaster".into());
    let fred_cfg = build_fred_config(&cfg).unwrap();
    match fred_cfg.server {
        ServerConfig::Sentinel {
            ref hosts,
            ref service_name,
            ..
        } => {
            assert_eq!(hosts.len(), 2);
            let mut ports: Vec<u16> = hosts.iter().map(|h| h.port).collect();
            ports.sort();
            assert_eq!(ports, vec![6379, 26380]);
            assert_eq!(service_name, "mymaster");
        }
        other => panic!("期望 Sentinel, got {:?}", other),
    }
}

#[test]
fn test_sentinel_requires_service_name() {
    let mut cfg = base();
    cfg.mode = ConnMode::Sentinel;
    cfg.service_name = None;
    assert!(build_fred_config(&cfg).is_err());
}

#[test]
fn test_master_slave_url() {
    // 主从模式：主库是 Centralized，从库不会进入主库 URL
    let mut cfg = base();
    cfg.mode = ConnMode::MasterSlave;
    cfg.nodes = vec![NodeSpec { host: "10.0.0.5".into(), port: 6381 }];
    let fred_cfg = build_fred_config(&cfg).unwrap();
    match fred_cfg.server {
        ServerConfig::Centralized { ref server } => {
            assert_eq!(server.port, 6379);
        }
        other => panic!("期望 Centralized, got {:?}", other),
    }
}
