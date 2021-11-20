use dbus::message::SignalArgs;
use dbus::nonblock::{self};
use dbus::Message;
use dbus_tokio::connection;
use futures_util::stream::StreamExt;
use log::Level;
use monistor_rs::codegen::MutterDisplayConfigMonitorsChanged;
use monistor_rs::update::update_monitors;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), anyhow::Error> {
    let result = try_main().await;
    if let Err(e) = &result {
        log::error!("Error running the program: {:#?}", e);
        log::error!("Error at backtrace: {:#?}", e.source());
    };
    result
}

pub async fn try_main() -> Result<(), anyhow::Error> {
    simple_logger::init_with_level(Level::Info).unwrap();
    // Connect to the D-Bus session bus (this is blocking, unfortunately).
    let (resource, conn) = connection::new_session_sync()?;
    let default_monitors = ["DP-4", "DP-0"];

    // The resource is a task that should be spawned onto a tokio compatible
    // reactor ASAP. If the resource ever finishes, you lost connection to D-Bus.
    tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    log::info!("Running first update to monitor");
    // Make a "proxy object" that contains the destination and path of our method call.
    let proxy = nonblock::Proxy::new(
        "org.gnome.Mutter.DisplayConfig",
        "/org/gnome/Mutter/DisplayConfig",
        Duration::from_secs(5),
        conn.clone(),
    );

    update_monitors(&default_monitors, &proxy).await?;

    log::info!("Entering streaming loop");
    let mr = MutterDisplayConfigMonitorsChanged::match_rule(None, None).static_clone();
    let (incoming_signal, stream) = conn.add_match(mr).await?.stream();
    let stream_future = stream.for_each(|(msg, _): (Message, ())| {
        log::info!(
            "Got signal from bus, updating monitor config: {:?}",
            msg.member()
        );
        async {
            if let Err(err) = update_monitors(&default_monitors, &proxy).await {
                log::warn!(
                    "Getting error from updating monitors {:?}. Retrying once",
                    err
                );
                update_monitors(&default_monitors, &proxy).await.unwrap();
            }
        }
    });

    stream_future.await;

    // Needed here to ensure the "incoming_signal" object is not dropped too early
    conn.remove_match(incoming_signal.token()).await?;

    unreachable!("Listening for dbus stream should never stop. Something went wrong")
}
