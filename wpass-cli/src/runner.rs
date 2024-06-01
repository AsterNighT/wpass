use crate::{fs, CmdArgumentMerged};
use anyhow::Result;
use log::debug;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::{future::Future, path::PathBuf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use wpass::WPassInstance;
use wpass::{parse_stdout, WPass};

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
}

impl Future for ArchiveFuture {
    type Output = Result<(String, Vec<PathBuf>)>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let result = self.wpass.extract_blindly(&self.target, &self.destination);
        if self.args.delete {
            if let Ok((ref output, ref extracted_archives)) = result {
                let info = parse_stdout(&output).unwrap();
                if info.volumes == extracted_archives.len() {
                    extracted_archives
                        .iter()
                        .try_for_each(std::fs::remove_file)
                        .unwrap();
                }
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
    current_jobs: Arc<Mutex<usize>>,
}

impl Scheduler {
    pub fn new(wpass: WPassInstance, args: &CmdArgumentMerged) -> Self {
        let (tx, rx) = mpsc::channel::<ArchiveFuture>(1000);
        let current_jobs = Arc::new(Mutex::new(0));
        let current_jobs_clone = current_jobs.clone();
        let max_jobs = args.max_thread;
        let task = tokio::spawn(async move {
            let mut receiver = rx;
            while let Some(archive) = receiver.recv().await {
                'outer: loop {
                    {
                        let mut current_jobs = current_jobs_clone.lock().unwrap();
                        if *current_jobs < max_jobs {
                            // Update the total complexity
                            *current_jobs += 1;

                            // Spawn the future
                            let current_jobs_clone = current_jobs_clone.clone();
                            tokio::spawn(async move {
                                let result = archive.await;
                                debug!("Job result: {:?}", result);
                                let mut current_jobs = current_jobs_clone.lock().unwrap();
                                *current_jobs -= 1;
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
            current_jobs,
            channel: tx,
            task,
        }
    }
    pub async fn handle(&mut self, files: &Vec<PathBuf>) -> Result<()> {
        for archive in files {
            let destination = fs::destination_from_file_path(
                &archive,
                &self.args.output,
                self.args.local,
                self.args.new_directory,
            )
            .unwrap();
            let job = ArchiveFuture {
                wpass: self.wpass.clone(),
                args: self.args.clone(),
                complexity: 0,
                target: archive.clone(),
                destination,
            };
            debug!("Handling job: {:?}", job);
            self.channel.send(job).await?;
        }
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
