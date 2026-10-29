//! The service's news: its two signals, and its coming and going.

use ferry_bus::StorageProxy;
use files_core::{CloudNews, DriveId};
use futures_util::StreamExt;

use super::decode;
use files_fs::News;

/// Tells `news` of every change of the service until a stream ends. Joins the service's watch
/// now and again each time the service appears (the join lasts until the connection to it
/// closes); a service that goes away or comes back is told as `CloudNews::Drives`, so the
/// caller asks for the drives again and finds none, or finds them back.
pub(super) async fn run(proxy: StorageProxy<'static>, news: News) {
    let Ok(mut drives) = proxy.receive_drives_changed().await else {
        return;
    };
    let Ok(mut items) = proxy.receive_items_changed().await else {
        return;
    };
    let Ok(mut owner) = proxy.inner().receive_owner_changed().await else {
        return;
    };
    // A service that is not there yet is joined when it appears.
    let _ = proxy.watch().await;
    loop {
        tokio::select! {
            Some(_) = drives.next() => news(CloudNews::Drives),
            Some(signal) = items.next() => {
                if let Ok(args) = signal.args() {
                    let drive = DriveId::new(args.drive().to_string());
                    let rows = args
                        .items()
                        .iter()
                        .filter_map(|(path, details)| decode::item(path, details).ok())
                        .collect();
                    news(CloudNews::Items(drive, rows));
                }
            }
            changed = owner.next() => match changed {
                Some(Some(_)) => {
                    let _ = proxy.watch().await;
                    news(CloudNews::Drives);
                }
                Some(None) => news(CloudNews::Drives),
                None => break,
            },
            else => break,
        }
    }
}
