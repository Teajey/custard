use anyhow::anyhow;
use camino::Utf8PathBuf;
use custard_lib::{
    collate,
    frontmatter_file::{self, Keeper},
    list, single,
};
use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};
use tracing::{debug, error, info};

#[derive(Serialize, Debug)]
#[serde(tag = "tag", content = "value")]
enum Result<T: Serialize> {
    Ok(T),
    InternalServerError(()),
}

// TODO: Ideally this would be statically generated
fn internal_server_error_bytes() -> Vec<u8> {
    let out_buf = rmp_serde::to_vec(&Result::<()>::InternalServerError(()))
        .expect("Result::InternalServerError does not serialize");

    debug!("Sending error bytes: {out_buf:x?}");

    out_buf
}

#[derive(Serialize, Debug)]
#[serde(tag = "tag", content = "value")]
enum Response<'a> {
    Single(Option<single::Response<'a>>),
    List(list::Response),
    Collate(Vec<String>),
}

#[derive(Deserialize, Debug)]
#[serde(tag = "tag", content = "value")]
enum Request<'a> {
    #[serde(borrow)]
    Single(single::Args<'a>),
    List(list::Args<'a>),
    Collate(collate::Args<'a>),
}

impl<'kep, 'req: 'kep> Request<'req> {
    fn process(self, keeper: &'kep Keeper) -> Response<'kep> {
        match self {
            Request::Single(args) => {
                let response = custard_lib::single::single(keeper, args);
                Response::Single(response)
            }
            Request::List(args) => {
                let response = custard_lib::list::query(keeper, args);
                Response::List(response)
            }
            Request::Collate(args) => {
                let response = custard_lib::collate::collate(keeper, args);
                Response::Collate(response)
            }
        }
    }
}

fn in_buf_2_out_buf(markdown_files: &frontmatter_file::keeper::ArcMutex, in_buf: &[u8]) -> Vec<u8> {
    debug!("Received bytes: {in_buf:x?}");
    let req = match rmp_serde::from_slice::<Request>(in_buf) {
        Ok(req) => req,
        Err(err) => {
            error!("stream request decode failed: {err}");
            return internal_server_error_bytes();
        }
    };

    let keeper = match markdown_files.lock() {
        Ok(keeper) => keeper,
        Err(err) => {
            error!("Failed to lock markdown files: {err}");
            return internal_server_error_bytes();
        }
    };
    let resp = req.process(&keeper);

    let out_buf = match resp {
        Response::Single(response) => rmp_serde::to_vec(&Result::Ok(response)),
        Response::List(list) => rmp_serde::to_vec(&Result::Ok(list)),
        Response::Collate(vec) => rmp_serde::to_vec(&Result::Ok(vec)),
    };

    match out_buf {
        Ok(out_buf) => {
            debug!("Sending bytes: {:x?}", out_buf);
            out_buf
        }
        Err(err) => {
            error!("Failed to serialize response: {err}");
            internal_server_error_bytes()
        }
    }
}

async fn accept_streams(
    markdown_files: frontmatter_file::keeper::ArcMutex,
    listener: UnixListener,
) {
    info!("listening for streams...");
    while let Ok((mut stream, _addr)) = listener.accept().await {
        debug!("accepted stream");
        let mf = markdown_files.clone();
        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            loop {
                // FIXME: I'm not confident in the integrity of the current code in this loop
                debug!("reading stream");
                let request_length = match stream.read_u32().await {
                    Ok(n) => n,
                    Err(err) => {
                        error!("Failed to read request length: {err}");
                        0
                    }
                };
                debug!("received request length: {request_length}");
                buf.resize(request_length as usize, 0);
                match stream.read_exact(&mut buf).await {
                    Ok(0) => {
                        debug!("stream terminated");
                        break;
                    }
                    Ok(n) => {
                        debug!("read {n} bytes");
                        let out_buf = in_buf_2_out_buf(&mf, &buf[..n]);
                        if let Err(err) = stream.write_all(&out_buf).await {
                            error!("stream write failed: {err}");
                        } else {
                            debug!("resolved request/response");
                            debug!("stream terminated");
                            break;
                        }
                    }
                    Err(err) => {
                        error!("stream read failed: {err}");
                    }
                }
            }
        });
    }
}

async fn run() -> anyhow::Result<()> {
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

    accept_streams(markdown_files, listener).await;

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod test {
    use super::Result;

    #[test]
    fn result_internal_server_error_bytes() {
        let bytes = rmp_serde::to_vec(&Result::<()>::InternalServerError(())).unwrap();
        let hex = format!("{bytes:x?}");
        assert_eq!(
            "[91, b3, 49, 6e, 74, 65, 72, 6e, 61, 6c, 53, 65, 72, 76, 65, 72, 45, 72, 72, 6f, 72]",
            hex
        );
    }

    #[test]
    fn result_ok_bytes() {
        let bytes = rmp_serde::to_vec(&Result::<u32>::Ok(1)).unwrap();
        let hex = format!("{bytes:x?}");
        assert_eq!("[92, a2, 4f, 6b, 1]", hex);
    }
}
