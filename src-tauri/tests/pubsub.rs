// Pub/Sub 与 MONITOR 冒烟测试（单机模式，127.0.0.1:6379）

use cache_manager_lib::model::{ConnConfig, ConnMode};
use cache_manager_lib::redisx::stream;
use cache_manager_lib::redisx::RedisManager;
use fred::prelude::*;
use fred::types::InfoKind;

fn single_config() -> ConnConfig {
    ConnConfig {
        id: "test-stream".into(),
        name: "流测试".into(),
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
async fn test_pubsub() {
    let manager = RedisManager::new();
    manager.connect(single_config()).await.expect("连接失败");

    // 用 tokio 通道模拟前端 Channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
    // 包装一个转发回调（这里直接验证后端 subscribe 不报错 + publish 成功）
    let handle = manager.get("test-stream").unwrap();
    let pool = RedisManager::pool_for(&handle, None).unwrap();
    let client = pool.clients()[0].clone();

    // 订阅频道
    client.subscribe(vec!["test-channel"]).await.expect("subscribe 失败");

    // 注册 on_message 回调（模拟 stream::subscribe 内部逻辑）
    let task = client.on_message(move |msg| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(serde_json::json!({
                "channel": msg.channel.to_string(),
                "value": match &msg.value {
                    Value::String(s) => s.to_string(),
                    Value::Bytes(b) => String::from_utf8_lossy(b.as_ref()).to_string(),
                    Value::Integer(i) => i.to_string(),
                    _ => format!("{:?}", msg.value),
                },
            }));
            Ok(())
        }
    });
    let _task = tokio::spawn(async move { let _ = task.await; });

    // 发布消息（用池中第二个连接，订阅连接处于订阅模式不能发布）
    let pub_client = pool.clients()[1].clone();
    let count: u64 = pub_client.publish("test-channel", "hello-pubsub").await.expect("publish 失败");
    assert_eq!(count, 1, "应有 1 个接收者");

    // 等待消息
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("超时未收到消息")
        .expect("通道关闭");
    assert_eq!(msg["channel"], "test-channel");
    assert!(msg["value"].as_str().unwrap().contains("hello-pubsub"));

    let _ = client.unsubscribe(vec!["test-channel"]).await;
    manager.disconnect("test-stream").await.unwrap();
}

#[tokio::test]
async fn test_monitor_start() {
    let manager = RedisManager::new();
    manager.connect(single_config()).await.expect("连接失败");
    let handle = manager.get("test-stream").unwrap();
    let pool = RedisManager::pool_for(&handle, None).unwrap();

    // 直接验证 monitor 模块能启动并收到一条命令
    let config = cache_manager_lib::redisx::build_fred_config(&single_config()).unwrap();
    let mut stream = fred::monitor::run(config).await.expect("monitor 启动失败");

    // 触发一条命令
    let _: () = pool.set("monitor:probe", "1", None, None, false).await.unwrap();
    let _: () = pool.del(vec!["monitor:probe"]).await.unwrap();

    let cmd = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        use futures::StreamExt;
        stream.next().await
    })
    .await
    .expect("超时")
    .expect("流结束");

    assert!(!cmd.command.is_empty());

    manager.disconnect("test-stream").await.unwrap();
}
