// Cluster 模式冒烟测试：3 节点集群 7000/7001/7002（无副本）

use cache_manager_lib::model::{ConnConfig, ConnMode, NodeSpec};
use cache_manager_lib::redisx::console;
use cache_manager_lib::redisx::keys;
use cache_manager_lib::redisx::server;
use cache_manager_lib::redisx::RedisManager;
use fred::prelude::*;

fn cluster_config() -> ConnConfig {
    ConnConfig {
        id: "test-cluster".into(),
        name: "集群测试".into(),
        mode: ConnMode::Cluster,
        host: "127.0.0.1".into(),
        port: 7000,
        username: None,
        password: None,
        database: None,
        nodes: vec![
            NodeSpec { host: "127.0.0.1".into(), port: 7001 },
            NodeSpec { host: "127.0.0.1".into(), port: 7002 },
        ],
        service_name: None,
        tls: false,
        connect_timeout_ms: 8000,
    }
}

#[tokio::test]
async fn test_cluster_mode() {
    let manager = RedisManager::new();

    // 连接
    manager.connect(cluster_config()).await.expect("集群连接失败");

    // 写入/读取（自动路由到对应槽位节点）
    let handle = manager.get("test-cluster").unwrap();
    let pool = RedisManager::pool_for(&handle, None).unwrap();
    let _: () = pool.set("{test}:a", "1", None, None, false).await.unwrap();
    let _: () = pool.set("{test}:b", "2", None, None, false).await.unwrap();
    let v: String = pool.get("{test}:a").await.unwrap();
    assert_eq!(v, "1");

    // 集群扫描（跨节点合并）
    let page = keys::scan_keys(&manager, "test-cluster", "0", "{test}:*", Some(100), None, None)
        .await
        .expect("集群扫描失败");
    assert!(
        page.keys.contains(&"{test}:a".to_string()) && page.keys.contains(&"{test}:b".to_string()),
        "扫描结果: {:?}",
        page.keys
    );

    // 无 hash tag 的 pattern 也应支持
    let page2 = keys::scan_keys(&manager, "test-cluster", "0", "*", Some(100), None, None)
        .await
        .expect("集群全量扫描失败");

    // 键信息
    let info = keys::key_info(&manager, "test-cluster", "{test}:a", None)
        .await
        .expect("key_info 失败");
    assert_eq!(info.r#type, "string");

    // 命令行
    let result = console::execute(&manager, "test-cluster", "GET {test}:b", None)
        .await
        .unwrap();
    assert!(result.ok, "console 失败: {:?}", result.error);
    assert_eq!(result.text.as_deref(), Some("2"));

    // 拓扑：应包含 3 个 master
    let nodes = server::get_topology(&manager, "test-cluster").await.unwrap();
    assert_eq!(nodes.iter().filter(|n| n.role == "master").count(), 3, "拓扑: {:?}", nodes);

    // 服务器信息
    let info = server::get_info(&manager, "test-cluster", Some("cluster".into())).await.unwrap();
    assert!(info.sections.iter().any(|s| s.name.eq_ignore_ascii_case("cluster")));

    // 清理
    let _: () = pool.del(vec!["{test}:a", "{test}:b"]).await.unwrap();
    manager.disconnect("test-cluster").await.unwrap();
}
