extern crate librsync;
use crate::deltasync::Callable;
use crate::synchrotrait::*;
use crate::threadmessage::ThreadMessage;
use librsync::{Delta, Patch, Signature};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::fs;
use std::io::{self, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::thread;

pub struct FsSync {
    paths: (PathBuf, PathBuf),
    max_retries: u32,
    rec_cb: Arc<Callable<String>>,
    sync_cb: Arc<Callable<String>>,
    max_threads: usize,
}

fn copy_file(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    let mut ofile = if !dest.exists() {
        fs::File::create(dest)?
    } else {
        fs::File::open(dest)?
    };
    io::copy(&mut fs::File::open(src)?, &mut ofile)?;
    Ok(())
}

fn cleanup(src: &PathBuf, dest: &PathBuf, pool: &Arc<ThreadPool>) -> Result<()> {
    if dest.is_dir() {
        if !src.exists() || src.is_file() {
            fs::remove_dir_all(dest)?;
            println!("Remove Dir {}", dest.as_path().to_str().unwrap_or("NONE"));
        } else {
            for e in fs::read_dir(dest)? {
                let entry = e?;
                let nsrc = src.join(PathBuf::from(entry.file_name()));
                let ndest = PathBuf::from(entry.path());
                let poolc = pool.clone();
                pool.install(move || {
                    cleanup(&nsrc, &ndest, &poolc).unwrap();
                });
            }
        }
    } else if dest.is_file() {
        if !src.exists() || src.is_dir() {
            println!("Remove {}", dest.as_path().to_str().unwrap_or("NONE"));
            fs::remove_file(dest)?;
        }
    }
    Ok(())
}

fn update_file(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    let ofile = fs::File::open(dest)?;
    let ifile = fs::File::open(src)?;
    let mut sig = Signature::new(&ofile).unwrap();
    let delta = Delta::new(&ifile, &mut sig).unwrap();
    let mut patch = Patch::new(&ofile, delta).unwrap();
    io::copy(&mut patch, &mut fs::File::open(dest)?)?;
    Ok(())
}

fn sync_file(src: &PathBuf, dest: &PathBuf, sc: &Arc<Callable<String>>) -> Result<()> {
    if dest.exists() {
        let imtime = fs::metadata(src)?.modified()?;
        let omtime = fs::metadata(dest)?.modified()?;
        let insize = fs::metadata(src)?.len();
        let osize = fs::metadata(dest)?.len();

        if imtime.gt(&omtime) && osize != insize {
            sc.run(format!(
                "Update {}",
                dest.as_path().to_str().unwrap_or("NONE")
            ));
            update_file(src, dest)?;
            fs::File::open(dest)?.set_permissions(fs::metadata(src)?.permissions())?;
        }
    } else {
        sc.run(format!(
            "Copy {}",
            dest.as_path().to_str().unwrap_or("NONE")
        ));
        copy_file(src, dest)?;
        fs::File::open(dest)?.set_permissions(fs::metadata(src)?.permissions())?;
    }

    Ok(())
}

fn sync_folder(src: &PathBuf, dest: &PathBuf, rc: &Arc<Callable<String>>) -> Result<()> {
    if !dest.exists() {
        fs::create_dir(dest)?;
    }
    fs::File::open(dest)?.set_permissions(fs::metadata(src)?.permissions())?;
    rc.run(format!(
        "Recursion {}",
        dest.as_path().to_str().unwrap_or("NONE")
    ));
    Ok(())
}

fn sync_tree(
    src: &PathBuf,
    dest: &PathBuf,
    rc: &Arc<Callable<String>>,
    sc: &Arc<Callable<String>>,
    pool: &Arc<ThreadPool>,
    tx: &SyncSender<ThreadMessage>,
    max_retries: u32,
) -> Result<()> {
    if src.is_dir() {
        sync_folder(src, dest, rc)?;
        for e in fs::read_dir(src)? {
            let entry = e?;
            let rcc = rc.clone();
            let scc = sc.clone();
            let poolc = pool.clone();
            let txc = tx.clone();
            let nsrc = entry.path();
            let ndest = dest.join(PathBuf::from(entry.file_name()));

            pool.install(move || {
                match sync_tree(&nsrc, &ndest, &rcc, &scc, &poolc, &txc, max_retries) {
                    Ok(()) => (),
                    Err(e) => txc
                        .send(ThreadMessage::from_error(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!(
                                "Src: {} Dest: {} => {}",
                                nsrc.as_path().to_str().unwrap_or("NONE"),
                                ndest.as_path().to_str().unwrap_or("NONE"),
                                e.to_string()
                            ),
                        )))
                        .unwrap(),
                }
            });
        }
    } else if src.is_file() {
        let mut run_retry = true;
        let mut retry_count = 0;

        while run_retry {
            match sync_file(src, dest, sc) {
                Ok(()) => {
                    run_retry = false;
                }
                Err(e) => {
                    retry_count += 1;
                    thread::sleep(std::time::Duration::from_secs(1));
                    if retry_count >= max_retries {
                        return Err(e);
                    }
                }
            }
        }
    }
    Ok(())
}

impl Synchro for FsSync {
    fn sync(&self) -> SynchroResult<SynchroState> {
        let rc = self.rec_cb.clone();
        let sc = self.sync_cb.clone();
        let pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(self.max_threads)
                .build()
                .unwrap(),
        );
        let src = PathBuf::from(&self.paths.0);
        let dest = PathBuf::from(&self.paths.1);
        let (tx, rx) = sync_channel(32);
        let run_error_capture = Arc::new(AtomicBool::new(true));
        let wrec = Arc::downgrade(&run_error_capture);
        let mr = self.max_retries;

        let handle = thread::spawn(move || -> Result<()> {
            sync_tree(&src, &dest, &rc, &sc, &pool, &tx, mr)?;
            cleanup(&src, &dest, &pool)?;
            run_error_capture.store(false, Ordering::SeqCst);
            Ok(())
        });

        let mut errors = Vec::new();
        while match wrec.upgrade() {
            Some(rec) => (*rec).load(Ordering::SeqCst),
            None => false,
        } {
            let tm = match rx.recv() {
                Ok(message) => match message {
                    ThreadMessage::Er(d) => Some(d.0),
                    _ => None,
                },
                Err(_) => None, // TODO Do something better here to avoid a "closed channel" error on termination
            };

            if tm.is_some() {
                errors.push(tm.unwrap());
            }
        }

        handle.join().unwrap()?;
        for e in errors {
            return Err(SynchroError::from(e));
        }

        Ok(SynchroState::Good(0))
    }
}

impl FsSync {
    pub fn new(src: &Path, dst: &Path) -> Self {
        FsSync {
            paths: (PathBuf::from(src), PathBuf::from(dst)),
            max_retries: 5,
            rec_cb: Arc::new(Callable::new(Box::new(|_: String| {}))),
            sync_cb: Arc::new(Callable::new(Box::new(|_: String| {}))),
            max_threads: 8,
        }
    }

    pub fn set_recursion_callback<F>(&mut self, cb: F)
    where
        F: 'static + Fn(String) + Send + Sync,
    {
        self.rec_cb = Arc::new(Callable::new(Box::new(cb)));
    }

    pub fn set_synchro_callback<F>(&mut self, cb: F)
    where
        F: 'static + Fn(String) + Send + Sync,
    {
        self.sync_cb = Arc::new(Callable::new(Box::new(cb)));
    }

    pub fn set_max_retries(&mut self, n: u32) {
        self.max_retries = n;
    }

    pub fn set_max_threads(&mut self, n: usize) {
        self.max_threads = n;
    }
}
