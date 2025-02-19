use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use custard_lib::single::Response;
use notify::{RecursiveMode, Watcher};
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};
use tracing::{debug, error, info};

#[derive(Deserialize)]
#[serde(tag = "tag", content = "value")]
enum Request {
    Get {
        name: String,
        #[serde(default)]
        sort_key: Option<String>,
        #[serde(default)]
        order_desc: bool,
    },
}

async fn run() -> Result<()> {
    let mut args = std::env::args();
    let socket_path = args
        .nth(1)
        .ok_or_else(|| anyhow!("Expected a socket path as a first argument"))?;
    if let Some(wd) = args.next() {
        std::env::set_current_dir(wd)?;
    }

    let socket_path = std::path::Path::new(&socket_path);
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let current_dir: Utf8PathBuf = std::env::current_dir()?.try_into()?;

    let keeper = custard_lib::frontmatter_file::Keeper::new(&current_dir)?;

    let markdown_files = custard_lib::frontmatter_file::keeper::ArcMutex::new(keeper);

    let mut watcher = notify::recommended_watcher(markdown_files.clone())?;

    watcher.watch(current_dir.as_std_path(), RecursiveMode::NonRecursive)?;

    let listener = UnixListener::bind(socket_path)?;

    info!("listening for streams...");
    while let Ok((mut stream, _addr)) = listener.accept().await {
        debug!("accepted stream");
        let mf = markdown_files.clone();
        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            loop {
                debug!("reading stream");
                match stream.read(&mut buf).await {
                    Ok(0) => {
                        debug!("stream terminated");
                        break;
                    }
                    Ok(n) => match rmp_serde::from_slice(&buf[..n]) {
                        Ok(Request::Get {
                            name,
                            sort_key,
                            order_desc,
                        }) => {
                            debug!("Received request: {name:?} {sort_key:?} {order_desc:?}");
                            let resp_buf = {
                                let keeper = mf.lock().unwrap();
                                let response = custard_lib::single::get(
                                    &keeper,
                                    &name,
                                    sort_key.as_deref(),
                                    order_desc,
                                );
                                debug!("Sending response: {response:?}");
                                rmp_serde::to_vec(&response).unwrap()
                            };
                            stream.write_all(&resp_buf).await.unwrap();
                        }
                        Err(err) => {
                            error!("stream decode failed: {err}");
                            let nun = rmp_serde::to_vec(&Option::<Response>::None).unwrap();
                            stream.write_all(&nun).await.unwrap();
                        }
                    },
                    Err(err) => {
                        error!("stream read failed: {err}");
                    }
                }
            }
        });
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
