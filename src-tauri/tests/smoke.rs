// 冒烟测试：连接本地 Redis，验证核心链路（连接 / 扫描 / 读写 / 命令执行）
// 需要本地 Redis 运行在 127.0.0.1:6379

use cache_manager_lib::model::{ConnConfig, ConnMode};
use cache_manager_lib::redisx::console;
use cache_manager_lib::redisx::keys;
use cache_manager_lib::redisx::server;
use cache_manager_lib::redisx::RedisManager;
use fred::prelude::*;

fn single_config() -> ConnConfig {
    ConnConfig {
        id: "test-single".into(),
        name: "本地测试".into(),
        mode: ConnMode::Single,
        host: "127.0.0.1".into(),
        port: 6379,
        username: None,
        password: None,
        database: Some(0),
        nodes: Vec::new(),
        service_name: None,
        tls: false,
        connect_timeout_ms: 5000,
    }
}

#[tokio::test]
async fn test_single_mode_full_flow() {
    let manager = RedisManager::new();

    // 1. 测试连接（不保存）
    let pong = RedisManager::test(&single_config()).await.expect("test 连接失败");
    assert_eq!(pong, "PONG");

    // 2. 建立连接
    manager.connect(single_config()).await.expect("建立连接失败");

    // 3. 准备数据
    let handle = manager.get("test-single").unwrap();
    let pool = RedisManager::pool_for(&handle, None).unwrap();
    let _: () = pool.del(vec!["smoke:k1", "smoke:h1", "smoke:l1"]).await.unwrap();
    let _: () = pool.set("smoke:k1", "hello world", None, None, false).await.unwrap();

    // 4. 扫描键
    let page = keys::scan_keys(&manager, "test-single", "0", "smoke:*", Some(100), None, None)
        .await
        .expect("scan 失败");
    assert!(page.keys.contains(&"smoke:k1".to_string()));
    assert_eq!(page.cursor, "0");

    // 5. 键信息
    let info = keys::key_info(&manager, "test-single", "smoke:k1", None)
        .await
        .expect("key_info 失败");
    assert_eq!(info.r#type, "string");
    assert_eq!(info.length, 11);

    // 6. 读取值
    let view = keys::get_value(&manager, "test-single", "smoke:k1", None)
        .await
        .expect("get_value 失败");
    match &view.payload {
        cache_manager_lib::redisx::keys::ValuePayload::String { value } => {
            assert_eq!(value.value, "hello world");
        }
        _ => panic!("类型错误"),
    }

    // 7. Hash
    let _ = keys::run_custom(pool, "HSET", vec!["smoke:h1".into(), "f1".into(), "v1".into()]).await.unwrap();
    let view = keys::get_value(&manager, "test-single", "smoke:h1", None).await.unwrap();
    match &view.payload {
        cache_manager_lib::redisx::keys::ValuePayload::Hash { fields, .. } => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].field.value, "f1");
        }
        _ => panic!("hash 类型错误"),
    }

    // 8. List
    let _ = keys::push_list(&manager, "test-single", "smoke:l1", vec!["a".into(), "b".into()], true).await.unwrap();
    let view = keys::get_value(&manager, "test-single", "smoke:l1", None).await.unwrap();
    match &view.payload {
        cache_manager_lib::redisx::keys::ValuePayload::List { values, .. } => {
            assert_eq!(values.len(), 2);
        }
        _ => panic!("list 类型错误"),
    }

    // 9. 命令行
    let result = console::execute(&manager, "test-single", "GET smoke:k1", None)
        .await
        .expect("execute 失败");
    assert!(result.ok);
    assert_eq!(result.text.as_deref(), Some("hello world"));

    let result = console::execute(&manager, "test-single", "SET smoke:k2 42", None)
        .await
        .expect("execute 失败");
    assert!(result.ok);

    // 10. 服务器信息
    let info = server::get_info(&manager, "test-single", None).await.expect("INFO 失败");
    assert!(!info.sections.is_empty());
    let has_server = info
        .sections
        .iter()
        .any(|s| s.name.eq_ignore_ascii_case("server"));
    assert!(has_server, "INFO 应包含 server 段");

    // 11. 配置
    let cfg = server::get_config(&manager, "test-single", "maxmemory*").await.unwrap();
    assert!(cfg.iter().any(|(k, _)| k == "maxmemory"));

    // 12. 拓扑（单机）
    let nodes = server::get_topology(&manager, "test-single").await.unwrap();
    assert!(!nodes.is_empty());
    assert_eq!(nodes[0].role, "master");

    // 13. 清理
    let n = keys::delete_keys(&manager, "test-single", vec!["smoke:k1".into(), "smoke:h1".into(), "smoke:l1".into(), "smoke:k2".into()]).await.unwrap();
    assert_eq!(n, 4);

    // 14. 断开
    manager.disconnect("test-single").await.unwrap();
    assert!(manager.get("test-single").is_err());
}
