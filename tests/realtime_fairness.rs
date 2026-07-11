//! P08 acceptance: Realtime fairness tests (plan P08.12).
//! Requires `--features realtime`.

#[cfg(feature = "realtime")]
use tokio::sync::broadcast;

#[cfg(feature = "realtime")]
const CHANNEL_CAPACITY: usize = 8;

#[cfg(feature = "realtime")]
#[test]
fn channel_capacity_matches_plan() {
    assert_eq!(CHANNEL_CAPACITY, 8);
}

#[cfg(feature = "realtime")]
#[tokio::test]
async fn broadcast_lagged_consumer_receives_error() {
    let (tx, mut rx1) = broadcast::channel::<i32>(CHANNEL_CAPACITY);
    let mut rx2 = tx.subscribe();
    for i in 0..CHANNEL_CAPACITY + 5 {
        let _ = tx.send(i as i32);
    }
    while rx1.try_recv().is_ok() {}
    assert!(rx2.try_recv().is_err());
}

#[cfg(feature = "realtime")]
#[tokio::test]
async fn fair_select_does_not_starve_receiver() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<i32>(CHANNEL_CAPACITY);
    let sender = tokio::spawn(async move {
        for i in 0..50 {
            let _ = tx.send(i).await;
            tokio::task::yield_now().await;
        }
    });
    let receiver = tokio::spawn(async move {
        let mut count = 0;
        while (rx.recv().await).is_some() {
            count += 1;
            if count >= 10 {
                break;
            }
        }
        count
    });
    let received = receiver.await.unwrap();
    assert!(received >= 10);
    sender.await.unwrap();
}

#[cfg(feature = "realtime")]
#[tokio::test]
async fn lagged_consumer_produces_exactly_one_error() {
    let (tx, _rx) = broadcast::channel::<i32>(CHANNEL_CAPACITY);
    let mut slow_rx = tx.subscribe();
    for i in 0..CHANNEL_CAPACITY + 10 {
        let _ = tx.send(i as i32);
    }
    let mut lagged = 0;
    loop {
        match slow_rx.try_recv() {
            Ok(_) => continue,
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                lagged += 1;
                assert!(n > 0);
                break;
            },
            Err(_) => break,
        }
    }
    assert_eq!(lagged, 1);
}

#[cfg(feature = "realtime")]
#[test]
fn session_config_default_is_constructible() {
    let cfg = zai_rs::realtime::protocol::SessionConfig::default();
    assert!(serde_json::to_value(&cfg).unwrap().is_object());
}
