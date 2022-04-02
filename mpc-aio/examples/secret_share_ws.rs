use mpc_aio::secret_share::{SecretShareMaster, SecretShareSlave};
use mpc_core::proto;
use mpc_core::secret_share::{SecretShare, SecretShareMessage};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{EncodedPoint, SecretKey};
use rand::thread_rng;
use tokio;
use tokio::net::UnixStream;
use tokio_util::codec::Framed;
use tracing::{info, instrument};
use tracing_subscriber;
use utils_aio::codec::ProstCodecDelimited;
use ws_stream_tungstenite::WsStream;

#[instrument(skip(stream, point))]
async fn master(stream: UnixStream, point: EncodedPoint) -> SecretShare {
    info!("Trying to connect");

    let ws = async_tungstenite::tokio::accept_async(stream)
        .await
        .expect("Master: Error during the websocket handshake occurred");

    info!("Websocket connected");

    let ws = WsStream::new(ws);

    let stream = Framed::new(
        ws,
        ProstCodecDelimited::<SecretShareMessage, proto::secret_share::SecretShareMessage>::default(
        ),
    );

    let mut master = SecretShareMaster::new(stream);

    let share = master.run(&point).await.unwrap();

    info!("Master key share: {:?}", share);

    share
}

#[instrument(skip(stream, point))]
async fn slave(stream: UnixStream, point: EncodedPoint) -> SecretShare {
    info!("Trying to connect");

    let (ws, _) = async_tungstenite::tokio::client_async("ws://local/ss", stream)
        .await
        .expect("Slave: Error during the websocket handshake occurred");

    info!("Websocket connected");

    let ws = WsStream::new(ws);

    let stream = Framed::new(
        ws,
        ProstCodecDelimited::<SecretShareMessage, proto::secret_share::SecretShareMessage>::default(
        ),
    );

    let mut slave = SecretShareSlave::new(stream);

    let share = slave.run(&point).await.unwrap();

    info!("Slave key share: {:?}", share);

    share
}

#[instrument]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Creating Unix Stream");
    let (unix_s, unix_r) = UnixStream::pair().unwrap();

    info!("Generating Master key");
    let master_point = SecretKey::random(&mut thread_rng())
        .public_key()
        .to_projective()
        .to_encoded_point(false);

    info!("Generating Slave key");
    let slave_point = SecretKey::random(&mut thread_rng())
        .public_key()
        .to_projective()
        .to_encoded_point(false);

    let master = master(unix_s, master_point);
    let slave = slave(unix_r, slave_point);

    let _ = tokio::join!(
        tokio::spawn(async move { master.await }),
        tokio::spawn(async move { slave.await })
    );
}
