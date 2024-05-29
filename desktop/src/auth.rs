use std::{
    borrow::Cow,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
};

use anyhow::Context;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

const EXIT: [u8; 4] = [1, 3, 3, 7];

/// The optional server config.
#[derive(Default, serde::Deserialize)]
pub struct OauthConfig {
    /// An array of hard-coded ports the server should try to bind to.
    /// This should only be used if your oauth provider does not accept wildcard localhost addresses.
    ///
    /// Default: Asks the system for a free port.
    pub ports: Option<Vec<u16>>,
    /// Optional static html string send to the user after being redirected.
    /// Keep it self-contained and as small as possible.
    ///
    /// Default: `"<html><body>Please return to the app.</body></html>"`.
    pub response: Option<Cow<'static, str>>,
}

pub fn start_with_config<F: FnMut(String) + Send + 'static>(
    config: OauthConfig,
    mut handler: F,
) -> Result<u16, std::io::Error> {
    let listener = match config.ports {
        Some(ports) => TcpListener::bind(
            ports
                .iter()
                .map(|p| SocketAddr::from(([127, 0, 0, 1], *p)))
                .collect::<Vec<SocketAddr>>()
                .as_slice(),
        ),
        None => TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))),
    }?;

    let port = listener.local_addr()?.port();

    thread::spawn(move || -> Result<(), anyhow::Error> {
        for conn in listener.incoming() {
			let conn = conn.inspect_err(|e| eprintln!("{e}"))?;
			let connection_result = handle_connection(conn, port).inspect_err(|e| eprintln!("{e}"))?;
			match connection_result {
				TcpConnectionOutcome::Exit => break,
				TcpConnectionOutcome::DidLoad(url) => {
					handler(url);
					break;
				},
				TcpConnectionOutcome::KeepAlive => (),
			}
        }

		Ok(())
    });

    Ok(port)
}

enum TcpConnectionOutcome {
    KeepAlive,
    Exit,
    DidLoad(String),
}

const CUSTOM_HTTP_AUTH_HEADER: &str = "x-Fieldz-Auth-Full-Url";

fn handle_connection(
    mut conn: TcpStream,
    port: u16,
) -> Result<TcpConnectionOutcome, anyhow::Error> {
    let mut buffer = [0; 4048];
    if let Err(io_err) = conn.read(&mut buffer) {
        eprintln!("Error reading incoming connection: {}", io_err.to_string());
    };
    if buffer[..4] == EXIT {
        return Ok(TcpConnectionOutcome::Exit);
    }

    const BASE_LEN: usize = 64;
    let mut headers = vec![httparse::EMPTY_HEADER; BASE_LEN];
    let mut request = httparse::Request::new(headers.as_mut_slice());

	request.parse(&buffer)?;

    let path = request.path.context("request is missing a path")?;

    if path == "/exit" {
        return Ok(TcpConnectionOutcome::Exit);
    };

    for header in &headers {
        if header.name == CUSTOM_HTTP_AUTH_HEADER {
            return Ok(TcpConnectionOutcome::DidLoad(
                String::from_utf8_lossy(header.value).into_owned(),
            ));
        }
    }

    let response = include_str!("./auth_redirect_page.html").replace(
        "<!--%%AUTH_SCRIPT%%-->",
        &format!(
            r#"
			<script>
				fetch("http://127.0.0.1:{port}", {{
					headers: {{
						"{CUSTOM_HTTP_AUTH_HEADER}": window.location.href,
					}},
				}})
			</script>
		"#
        ),
    );

    conn.write_all(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            response.len(),
            response
        )
        .as_bytes(),
    )?;

    conn.flush()?;

    Ok(TcpConnectionOutcome::KeepAlive)
}

/// Stops the currently running server behind the provided port without executing the handler.
/// Alternatively you can send a request to http://127.0.0.1:port/exit
///
/// # Errors
///
/// - Returns `std::io::Error` if the server couldn't be reached.
pub fn cancel(port: u16) -> Result<(), std::io::Error> {
    // Using tcp instead of something global-ish like an AtomicBool,
    // so we don't have to dive into the set_nonblocking madness.
    let mut stream = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port)))?;
    stream.write_all(&EXIT)?;
    stream.flush()?;

    Ok(())
}

mod plugin_impl {
    use tauri::{Manager, Runtime, Window};

    #[tauri::command]
    pub(crate) fn start<R: Runtime>(
        window: Window<R>,
        config: Option<super::OauthConfig>,
    ) -> Result<u16, String> {
        let mut config = config.unwrap_or_default();
        if config.response.is_none() {
            config.response = window
                .config()
                .plugins
                .0
                .get("oauth")
                .map(|v| v.as_str().unwrap().to_string().into());
        }

        super::start_with_config(config, move |url| match url::Url::parse(&url) {
            Ok(_) => {
                if let Err(emit_err) = window.emit("oauth://url", url) {
                    eprintln!("Error emitting oauth://url event: {}", emit_err)
                };
            }
            Err(err) => {
                if let Err(emit_err) = window.emit("oauth://invalid-url", err.to_string()) {
                    eprintln!("Error emitting oauth://invalid-url event: {}", emit_err)
                };
            }
        })
        .map_err(|err| err.to_string())
    }

    #[tauri::command]
    pub(crate) fn cancel(port: u16) -> Result<(), String> {
        super::cancel(port).map_err(|err| err.to_string())
    }
}

/// Initializes the tauri plugin.
/// Only use this if you need the JavaScript APIs.
///
/// Note for the `start()` command: If `response` is not provided it will fall back to the config
/// in tauri.conf.json if set and will fall back to the library's default, see [`OauthConfig`].
#[must_use]
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("oauth")
        .invoke_handler(tauri::generate_handler![
            plugin_impl::start,
            plugin_impl::cancel
        ])
        .build()
}
