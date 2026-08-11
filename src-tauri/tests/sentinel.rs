// Sentinel 模式冒烟测试：sentinel 26379 监控 mymaster (127.0.0.1:6380, replica 6381)

use cache_manager_lib::model::{ConnConfig, ConnMode, NodeSpec};
use cache_manager_lib::redisx::console;
use cache_manager_lib::redisx::keys;
use cache_manager_lib::redisx::server;
use cache_manager_lib::redisx::RedisManager;
use fred::prelude::*;

fn sentinel_config() -> ConnConfig {
    ConnConfig {
        id: "test-sentinel".into(),
        name: "哨兵测试".into(),
        mode: ConnMode::Sentinel,
        host: "127.0.0.1".into(),
        port: 26379,
        username: None,
        password: None,
        database: Some(0),
        nodes: vec![],
        service_name: Some("mymaster".into()),
        tls: false,
        connect_timeout_ms: 5000,
    }
}

#[tokio::test]
async fn test_sentinel_mode() {
    let manager = RedisManager::new();

    // 连接（通过 sentinel 发现主节点）
    manager.connect(sentinel_config()).await.expect("哨兵连接失败");

    // 读写
    let handle = manager.get("test-sentinel").unwrap();
    let pool = RedisManager::pool_for(&handle, None).unwrap();
    let _: () = pool.set("sentinel:key", "via-sentinel", None, None, false).await.unwrap();
    let v: String = pool.get("sentinel:key").await.unwrap();
    assert_eq!(v, "via-sentinel");

    // 扫描
    let page = keys::scan_keys(&manager, "test-sentinel", "0", "sentinel:*", Some(100), None, None)
        .await
        .expect("扫描失败");
    assert!(page.keys.contains(&"sentinel:key".to_string()));

    // 命令行
    let result = console::execute(&manager, "test-sentinel", "GET sentinel:key", None)
        .await
        .unwrap();
    assert!(result.ok);
    assert_eq!(result.text.as_deref(), Some("via-sentinel"));

    // 拓扑：应包含 master / replica / sentinel
    let nodes = server::get_topology(&manager, "test-sentinel").await.unwrap();
    assert!(nodes.iter().any(|n| n.role == "master"), "拓扑中缺少 master: {:?}", nodes);
    assert!(nodes.iter().any(|n| n.role == "replica"), "拓扑中缺少 replica");
    assert!(nodes.iter().any(|n| n.role == "sentinel"), "拓扑中缺少 sentinel");

    // 清理
    let _: () = pool.del(vec!["sentinel:key"]).await.unwrap();
    manager.disconnect("test-sentinel").await.unwrap();
}
