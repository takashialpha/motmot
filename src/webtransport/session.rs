use std::sync::Arc;

use bytes::Bytes;
use h3::quic::BidiStream;
use h3_webtransport::server::{self, WebTransportSession};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{error, info};

use super::error::WebTransportError;
use crate::config::AppConfig;

pub async fn handle_session(
    session: WebTransportSession<h3_quinn::Connection, Bytes>,
    _config: Arc<AppConfig>,
    server_name: Arc<String>,
) -> Result<(), WebTransportError> {
    let session_id_dbg = format!("{:?}", session.session_id());

    info!(
        server = %server_name,
        session_id = %session_id_dbg,
        "webtransport_session_start"
    );

    let mut datagram_reader = session.datagram_reader();
    let mut datagram_sender = session.datagram_sender();

    loop {
        tokio::select! {
            datagram = datagram_reader.read_datagram() => {
                let datagram = match datagram {
                    Ok(d) => d,
                    Err(e) => {
                        error!(
                            server = %server_name,
                            session_id = %session_id_dbg,
                            error = %e,
                            "datagram_read_failed"
                        );
                        break;
                    }
                };

                info!(
                    server = %server_name,
                    session_id = %session_id_dbg,
                    bytes = datagram.payload().len(),
                    "datagram_received"
                );

                if let Err(e) = datagram_sender.send_datagram(datagram.into_payload()) {
                    error!(
                        server = %server_name,
                        session_id = %session_id_dbg,
                        error = %e,
                        "datagram_send_failed"
                    );
                }
            }

            uni_stream = session.accept_uni() => {
                let (stream_id, recv) = match uni_stream {
                    Ok(Some(s)) => s,
                    Ok(None) => continue,
                    Err(e) => {
                        error!(
                            server = %server_name,
                            session_id = %session_id_dbg,
                            error = %e,
                            "uni_stream_accept_failed"
                        );
                        break;
                    }
                };

                let stream_id_dbg = format!("{:?}", stream_id);
                let stream_id_for_task = stream_id_dbg.clone();

                info!(
                    server = %server_name,
                    session_id = %session_id_dbg,
                    stream_id = %stream_id_dbg,
                    "uni_stream_accepted"
                );

                let send = match session.open_uni(stream_id).await {
                    Ok(s) => s,
                    Err(e) => {
                        error!(
                            server = %server_name,
                            session_id = %session_id_dbg,
                            stream_id = %stream_id_dbg,
                            error = %e,
                            "uni_stream_open_failed"
                        );
                        continue;
                    }
                };

                let server_name = server_name.clone();
                tokio::spawn(async move {
                    if let Err(e) = echo_stream(send, recv, &server_name, stream_id_for_task).await {
                        error!(
                            server = %server_name,
                            stream_id = %stream_id_dbg,
                            error = %e,
                            "uni_stream_echo_failed"
                        );
                    }
                });
            }

            bidi_stream = session.accept_bi() => {
                match bidi_stream {
                    Ok(Some(server::AcceptedBi::BidiStream(stream_id, stream))) => {
                        let stream_id_dbg = format!("{:?}", stream_id);
                        let stream_id_for_task = stream_id_dbg.clone();

                        info!(
                            server = %server_name,
                            session_id = %session_id_dbg,
                            stream_id = %stream_id_dbg,
                            "bidi_stream_accepted"
                        );

                        let (send, recv) = BidiStream::split(stream);
                        let server_name = server_name.clone();

                        tokio::spawn(async move {
                            if let Err(e) = echo_stream(send, recv, &server_name, stream_id_for_task).await {
                                error!(
                                    server = %server_name,
                                    stream_id = %stream_id_dbg,
                                    error = %e,
                                    "bidi_stream_echo_failed"
                                );
                            }
                        });
                    }
                    Ok(Some(server::AcceptedBi::Request(_, _))) => {
                        error!(
                            server = %server_name,
                            session_id = %session_id_dbg,
                            "unexpected_http_request_in_webtransport_session"
                        );
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        error!(
                            server = %server_name,
                            session_id = %session_id_dbg,
                            error = %e,
                            "bidi_stream_accept_failed"
                        );
                        break;
                    }
                }
            }

            else => break,
        }
    }

    info!(
        server = %server_name,
        session_id = %session_id_dbg,
        "webtransport_session_complete"
    );

    Ok(())
}

async fn echo_stream<S, R>(
    mut send: S,
    mut recv: R,
    server_name: &str,
    stream_id: String,
) -> Result<(), WebTransportError>
where
    S: AsyncWrite + Unpin,
    R: AsyncRead + Unpin,
{
    info!(
        server = server_name,
        stream_id = %stream_id,
        "stream_echo_start"
    );

    let mut buf = Vec::new();
    let bytes = recv
        .read_to_end(&mut buf)
        .await
        .map_err(WebTransportError::Io)?;

    send.write_all(&buf).await.map_err(WebTransportError::Io)?;
    send.shutdown().await.map_err(WebTransportError::Io)?;

    info!(
        server = server_name,
        stream_id = %stream_id,
        bytes = bytes,
        "stream_echo_complete"
    );

    Ok(())
}
