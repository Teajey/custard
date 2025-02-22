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
enum Result<'a> {
    Ok(Response<'a>),
    InternalServerError,
}

// TODO: Ideally this would be statically generated
fn internal_server_error_bytes() -> Vec<u8> {
    rmp_serde::to_vec(&Result::InternalServerError)
        .expect("Result::InternalServerError does not serialize")
}

#[derive(Serialize, Debug)]
enum Response<'a> {
    Single(Option<single::Response<'a>>),
    List(Vec<frontmatter_file::Short>),
    Collate(Vec<String>),
}

#[derive(Deserialize, Debug)]
#[serde(tag = "tag", content = "value")]
enum Request<'a> {
    #[serde(borrow)]
    SingleGet(single::Get<'a>),
    SingleQuery(single::Query<'a>),
    ListGet(list::Get<'a>),
    ListQuery(list::Query<'a>),
    CollateGet(collate::Get<'a>),
    CollateQuery(collate::Query<'a>),
}

impl<'kep, 'req: 'kep> Request<'req> {
    fn process(self, keeper: &'kep Keeper) -> Response<'kep> {
        match self {
            Request::SingleGet(args) => {
                let response = custard_lib::single::get(keeper, args);
                Response::Single(response)
            }
            Request::SingleQuery(args) => {
                let response = custard_lib::single::query(keeper, args);
                Response::Single(response)
            }
            Request::ListGet(args) => {
                let response = custard_lib::list::get(keeper, args);
                Response::List(response)
            }
            Request::ListQuery(args) => {
                let response = custard_lib::list::query(keeper, args);
                Response::List(response)
            }
            Request::CollateGet(args) => {
                let response = custard_lib::collate::get(keeper, args);
                Response::Collate(response)
            }
            Request::CollateQuery(args) => {
                let response = custard_lib::collate::query(keeper, args);
                Response::Collate(response)
            }
        }
    }
}

fn in_buf_2_out_buf(markdown_files: &frontmatter_file::keeper::ArcMutex, in_buf: &[u8]) -> Vec<u8> {
    let req = match rmp_serde::from_slice::<Request>(in_buf) {
        Ok(req) => req,
        Err(err) => {
            error!("stream decode failed: {err}");
            return internal_server_error_bytes();
        }
    };
    debug!("Received request: {req:?}");

    let keeper = match markdown_files.lock() {
        Ok(keeper) => keeper,
        Err(err) => {
            error!("Failed to lock markdown files: {err}");
            return internal_server_error_bytes();
        }
    };
    let resp = req.process(&keeper);
    debug!("Sending response: {resp:?}");

    match rmp_serde::to_vec(&Result::Ok(resp)) {
        Ok(out_buf) => out_buf,
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
                debug!("reading stream");
                match stream.read(&mut buf).await {
                    Ok(0) => {
                        debug!("stream terminated");
                        break;
                    }
                    Ok(n) => {
                        let out_buf = in_buf_2_out_buf(&mf, &buf[..n]);
                        let Err(err) = stream.write_all(&out_buf).await else {
                            continue;
                        };
                        error!("stream write failed: {err}");
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
    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
