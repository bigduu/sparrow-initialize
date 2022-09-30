use std::env::temp_dir;
use std::error::Error;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::rc::Rc;
use std::sync::Arc;

use futures::future::join_all;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::Instant;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let dir = Arc::new(temp_dir().display().to_string());
    let instant = Instant::now();
    let remote_url = read_address().await?;
    clone_from_remote(&dir, remote_url).await;
    install_each_repository(&dir).await;
    let elapsed = instant.elapsed().as_secs_f64();
    info!("Done .... use {} seconds",elapsed);
    Ok(())
}

async fn read_address<>() -> Result<Vec<String>, Box<dyn Error>> {
    let result = File::open("config").await?;
    let reader = BufReader::new(result);
    let mut lines = reader.lines();
    let mut results: Vec<String> = vec![];
    while let Some(line) = lines.next_line().await? {
        results.push(line);
    }
    Ok(results)
}

async fn install_each_repository(dir: &Arc<String>) {
    let mvn_install_order_list: Vec<&str> = vec!["sparrow-bom", "sparrow-shell", "sparrow-zoo-bom", "sparrow-passport-ddd"];
    for repository in mvn_install_order_list.iter() {
        let mut buf = PathBuf::from(dir.to_string());
        buf.push(repository);
        let rc = Rc::new(buf.display().to_string());
        let project_dir = rc.as_str();
        info!("Start to install {}, may take a while, because the maven need download the dependency.",repository);
        if cfg!(target_os = "windows") {
            window_mvn_install(project_dir).await
        } else {
            unix_like_mvn_install(project_dir).await
        }
    }
}

async fn unix_like_mvn_install(project_dir: &str) {
    let args = vec!["-c", "mvn", "-T4", "clean", "install", "-DskipTests=true"];
    execute(project_dir, "sh", args).await;
}

async fn window_mvn_install(project_dir: &str) {
    let args = vec!["/c", "mvn", "-T4", "clean", "install", "-DskipTests=true"];
    execute(project_dir, "cmd", args).await;
}

async fn clone_from_remote(dir: &Arc<String>, remote_url: Vec<String>) {
    let mut join_list = vec![];
    for repository in remote_url.into_iter() {
        let dir = Arc::clone(dir);
        let handle = tokio::spawn(async move {
            let args = vec!["clone", repository.as_str()];
            execute(dir.as_str(), "git", args).await;
        });
        join_list.push(handle);
    }
    join_all(join_list).await;
}

async fn execute(dir: &str, command: &str, args: Vec<&str>) {
    info!("Start to execute {} {:?}",command,&args);
    match Command::new(command)
        .args(&args)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await {
        Ok(out) => {
            io::stdout().write_all(&out.stdout).unwrap();
            io::stderr().write_all(&out.stderr).unwrap();
            info!("Finished {} ,{:?}",command,&args)
        }
        Err(_) => {
            error!("{},{:?} fail", command,&args)
        }
    }
}
