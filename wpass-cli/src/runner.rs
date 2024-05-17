use crate::{fs, CmdArgumentMerged};
use anyhow::Result;
use log::debug;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::{future::Future, path::PathBuf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use wpass::WPass;
use wpass::WPassInstance;

#[derive(Debug, Clone)]
pub struct Archive {
    pub path: PathBuf,
    pub password: String,
    pub size: usize,
}

#[derive(Debug, Clone)]
struct ArchiveFuture {
    wpass: WPassInstance,
    args: CmdArgumentMerged,
    complexity: usize,
    target: PathBuf,
    destination: PathBuf,
    password: String,
}

impl Future for ArchiveFuture {
    type Output = Result<Vec<PathBuf>>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let result = self.wpass.extract_with_password(&self.target, &self.destination, &self.password);
        if self.args.delete {
            if let Ok(ref extracted_archives) = result {
                extracted_archives
                    .iter()
                    .try_for_each(std::fs::remove_file)?;
            }
        }
        Poll::Ready(result)
    }
}

impl ArchiveFuture {
    fn complexity(&self) -> usize {
        self.complexity
    }
}

pub struct Scheduler {
    task: JoinHandle<Result<()>>,
    channel: mpsc::Sender<ArchiveFuture>,
    wpass: WPassInstance,
    args: CmdArgumentMerged,
    current_complexity: Arc<Mutex<(usize, usize)>>,
}

impl Scheduler {
    pub fn new(wpass: WPassInstance, args: &CmdArgumentMerged) -> Self {
        let (tx, rx) = mpsc::channel::<ArchiveFuture>(1000);
        let current_complexity = Arc::new(Mutex::new((0, 0)));
        let current_complexity_clone = current_complexity.clone();
        let max_complexity = (args.max_ram, args.max_thread);
        let task = tokio::spawn(async move {
            let mut receiver = rx;
            while let Some(archive) = receiver.recv().await {
                let complexity = archive.complexity();
                'outer: loop {
                    {
                        let mut current_complexity = current_complexity_clone.lock().unwrap();
                        if current_complexity.0 + complexity <= max_complexity.0
                            && current_complexity.1 < max_complexity.1
                        {
                            // Update the total complexity
                            current_complexity.0 += complexity;
                            current_complexity.1 += 1;

                            // Spawn the future
                            let current_complexity_clone = current_complexity_clone.clone();
                            tokio::spawn(async move {
                                let result = archive.await;
                                debug!("Job result: {:?}", result);
                                let mut current_complexity =
                                    current_complexity_clone.lock().unwrap();
                                current_complexity.0 -= complexity;
                                current_complexity.1 -= 1;
                            });
                            break 'outer;
                        }
                    }
                    tokio::task::yield_now().await;
                }
            }
            Ok(())
        });
        Self {
            args: args.clone(),
            wpass,
            current_complexity,
            channel: tx,
            task,
        }
    }
    pub async fn handle(&mut self, files: &Vec<Archive>) -> Result<()>{
        for archive in files {
            let destination = fs::destination_from_file_path(
                &archive.path,
                &self.args.output,
                self.args.local,
                self.args.new_directory,
            )
            .unwrap();
            let job = ArchiveFuture {
                wpass: self.wpass.clone(),
                args: self.args.clone(),
                complexity: archive.size,
                password: archive.password.clone(),
                target: archive.path.clone(),
                destination,
            };
            debug!("Handling job: {:?}", job);
            self.channel.send(job).await?;
        };
        Ok(())
    }
    pub async fn join(self) -> Result<()> {
        drop(self.channel);
        self.task.await?
    }
}

// fn finalize(args: &CmdArgumentMerged, extracted_archives: &Vec<PathBuf>) -> Result<()> {
//     if args.delete {
//         extracted_archives
//             .iter()
//             .try_for_each(std::fs::remove_file)?;
//     }
// }
