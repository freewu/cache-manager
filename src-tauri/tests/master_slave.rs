// 主从模式冒烟测试：master 6380, replica 6381
// 需要先启动：redis-server master.conf / replica.conf

use cache_manager_lib::model::{ConnConfig, ConnMode, NodeSpec};
use cache_manager_lib::redisx::console;
use cache_manager_lib::redisx::keys;
use cache_manager_lib::redisx::server;
use cache_manager_lib::redisx::RedisManager;
use fred::prelude::*;

fn master_slave_config() -> ConnConfig {
    ConnConfig {
        id: "test-ms".into(),
        name: "主从测试".into(),
        mode: ConnMode::MasterSlave,
        host: "127.0.0.1".into(),
        port: 6380,
        username: None,
        password: None,
        database: Some(0),
        nodes: vec![NodeSpec {
            host: "127.0.0.1".into(),
            port: 6381,
        }],
        service_name: None,
        tls: false,
        connect_timeout_ms: 5000,
    }
}

#[tokio::test]
async fn test_master_slave_mode() {
    let manager = RedisManager::new();

    // 连接（含从库池）
    manager.connect(master_slave_config()).await.expect("主从连接失败");

    // 主库写入
    let handle = manager.get("test-ms").unwrap();
    let master_pool = RedisManager::pool_for(&handle, None).unwrap();
    let _: () = master_pool.set("ms:key", "from-master", None, None, false).await.unwrap();

    // 等复制
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 从库读取
    let replica_pool = RedisManager::pool_for(&handle, Some(0)).unwrap();
    let v: String = replica_pool.get("ms:key").await.unwrap();
    assert_eq!(v, "from-master");

    // 从库扫描
    let page = keys::scan_keys(&manager, "test-ms", "0", "ms:*", Some(100), None, Some(0))
        .await
        .expect("从库扫描失败");
    assert!(page.keys.contains(&"ms:key".to_string()));

    // 从库读值
    let view = keys::get_value(&manager, "test-ms", "ms:key", Some(0))
        .await
        .expect("从库读取失败");
    match &view.payload {
        cache_manager_lib::redisx::keys::ValuePayload::String { value } => {
            assert_eq!(value.value, "from-master");
        }
        _ => panic!("类型错误"),
    }

    // 从库执行只读命令
    let result = console::execute(&manager, "test-ms", "GET ms:key", Some(0))
        .await
        .unwrap();
    assert!(result.ok);
    assert_eq!(result.text.as_deref(), Some("from-master"));

    // 拓扑：应包含 master + replica
    let nodes = server::get_topology(&manager, "test-ms").await.unwrap();
    assert!(nodes.iter().any(|n| n.role == "master"));
    assert!(nodes.iter().any(|n| n.role == "replica"));

    // 清理
    let _: () = master_pool.del(vec!["ms:key"]).await.unwrap();
    manager.disconnect("test-ms").await.unwrap();
}
