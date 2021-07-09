extern crate librsync;
extern crate rayon;
use crate::deltasync::Callable;
use crate::net::host::Host;
use crate::synchrotrait::*;
use crate::threadmessage::ThreadMessage;
use librsync::{Delta, Patch, Signature};
use rayon::{ThreadPool, ThreadPoolBuilder};
use ssh2::{ErrorCode, Session, Sftp};
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use std::net::TcpStream;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::UNIX_EPOCH;

fn remrmr(d: &PathBuf, sftp: &Arc<Sftp>, meta: Option<ssh2::FileStat>) -> Result<()> {
    let rmr = move || -> Result<()> {
        let entries = sftp.readdir(d)?;
        for e in entries {
            remrmr(&e.0, sftp, Some(e.1))?;
        }
        sftp.rmdir(d)?;
        Ok(())
    };

    match meta {
        Some(m) => {
            if m.is_dir() {
                rmr()?;
            } else if m.is_file() {
                sftp.unlink(d)?;
            }
        }
        None => {
            rmr()?;
        }
    }
    Ok(())
}

fn cleanup(
    src: &PathBuf,
    dest: &PathBuf,
    sftp: &Arc<Sftp>,
    pool: &Arc<ThreadPool>,
    tx: &SyncSender<ThreadMessage>,
) -> Result<()> {
    match sftp.readdir(dest) {
        Ok(vec) => {
            for v in vec {
                let src_match = src.join(PathBuf::from(&v.0.file_name().unwrap()));
                if v.1.is_dir() {
                    let ndest = dest.join(&v.0);
                    if !src_match.exists() {
                        println!(
                            "Cleanup Dir {}",
                            &ndest.as_path().to_str().unwrap_or("None")
                        );
                        remrmr(&ndest, sftp, None)?;
                    } else {
                        let poolc = pool.clone();
                        let txc = tx.clone();
                        pool.install(move || {
                            match cleanup(&src_match, &ndest, sftp, &poolc, &txc) {
                                Ok(()) => (),
                                Err(e) => {
                                    txc.send(ThreadMessage::from_error(e)).unwrap();
                                }
                            }
                        });
                    }
                } else {
                    let poolc = pool.clone();
                    let txc = tx.clone();
                    pool.install(move || {
                        match cleanup(&src_match, &dest.join(&v.0), sftp, &poolc, &txc) {
                            Ok(()) => (),
                            Err(e) => {
                                txc.send(ThreadMessage::from_error(e)).unwrap();
                            }
                        }
                    });
                }
            }
        }
        Err(_) => match sftp.open(dest) {
            Ok(_) => {
                if !src.exists() {
                    println!("Cleanup {}", dest.as_path().to_str().unwrap_or("None"));
                    sftp.unlink(dest)?;
                }
            }
            Err(_) => (),
        },
    }
    Ok(())
}

fn stream_contents_keepalive<I, O, F>(
    input_stream: &mut I,
    output_stream: &mut O,
    session: &Arc<Session>,
    callback: F,
) -> Result<()>
where
    I: std::io::Read,
    O: std::io::Write,
    F: Fn(usize) + 'static,
{
    let mut start = std::time::Instant::now();
    let keepalive_run = Arc::new(AtomicBool::new(true));
    let send_keepalive = Arc::new(AtomicBool::new(false));
    let mut run_copy = true;

    let sessc = session.clone();
    let krc = keepalive_run.clone();
    let skc = send_keepalive.clone();
    let keepalive = thread::spawn(move || -> Result<()> {
        while krc.load(Ordering::SeqCst) {
            if skc.load(Ordering::SeqCst) {
                sessc.keepalive_send()?;
                skc.store(false, Ordering::SeqCst);
            }
            thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    });

    while run_copy {
        let mut buffer = [0; 8192];

        let bytes_read = input_stream.read(&mut buffer)?;
        let bytes_written = output_stream.write(&buffer[0..bytes_read])?;
        run_copy = if bytes_read == 0 { false } else { true };
        callback(bytes_written);
        if start.elapsed().gt(&Duration::from_secs(30)) {
            send_keepalive.store(true, Ordering::SeqCst);
            start = std::time::Instant::now();
            thread::sleep(Duration::from_millis(10));
        }
    }
    keepalive_run.store(false, Ordering::SeqCst);
    match keepalive.join() {
        Ok(result) => match result {
            Ok(()) => (),
            Err(e) => return Err(e),
        },
        Err(_) => return Err(Error::new(ErrorKind::Other, "Thread panicked !")),
    }

    Ok(())
}

fn gen_file_stat(path: &PathBuf) -> ssh2::FileStat {
    let meta = match fs::metadata(path) {
        Ok(m) => Some(m),
        Err(_) => None,
    };
    match meta {
        Some(m) => ssh2::FileStat {
            size: Some(m.len()),
            atime: match m
                .accessed()
                .unwrap_or(std::time::SystemTime::now())
                .duration_since(UNIX_EPOCH)
            {
                Ok(t) => Some(t.as_secs()),
                Err(_) => None,
            },
            mtime: match m
                .modified()
                .unwrap_or(std::time::SystemTime::now())
                .duration_since(UNIX_EPOCH)
            {
                Ok(t) => Some(t.as_secs()),
                Err(_) => None,
            },
            uid: Some(m.uid()),
            gid: Some(m.gid()),
            perm: Some(m.mode()),
        },
        None => ssh2::FileStat {
            size: None,
            atime: None,
            mtime: None,
            uid: Some(0),
            gid: Some(0),
            perm: Some(644),
        },
    }
}

pub fn upload_file(src: &PathBuf, dest: &PathBuf, session: &Arc<Session>) -> Result<()> {
    let mut ifile = fs::File::open(src)?;
    let mut chan = session.channel_session()?;
    chan.exec(&format!(
        "dd of=\"{}\"",
        dest.as_path().to_str().unwrap_or("tmp.lost")
    ))?;
    let mut stream = chan.stream(0);

    stream_contents_keepalive(&mut ifile, &mut stream, session, move |_| {})?;

    Ok(())
}

fn update_file(
    src: &PathBuf,
    dest: &PathBuf,
    sftp: &Arc<Sftp>,
    session: &Arc<Session>,
) -> Result<()> {
    let ifile = fs::File::open(src)?;
    let mut sig = Signature::new(sftp.open(dest)?).unwrap();
    let delta = Delta::new(&ifile, &mut sig).unwrap();
    let mut patch = Patch::new(sftp.open(dest)?, delta).unwrap();
    let mut chan = session.channel_session()?;
    chan.exec(&format!(
        "dd of=\"{}\"",
        dest.as_path().to_str().unwrap_or("tmp.lost")
    ))?;
    let mut stream = chan.stream(0);
    stream_contents_keepalive(&mut patch, &mut stream, session, |_| {})?;
    Ok(())
}

fn sync_file(
    src: &PathBuf,
    dest: &PathBuf,
    sc: &Arc<Callable<String>>,
    sftp: &Arc<Sftp>,
    session: &Arc<Session>,
) -> Result<()> {
    match sftp.open(dest) {
        Ok(mut ofile) => {
            let ostat = ofile.stat()?;
            let istat = gen_file_stat(src);
            if istat.mtime.unwrap_or(0) > ostat.mtime.unwrap_or(0) {
                sc.run(format!(
                    "Update: {}",
                    dest.as_path().to_str().unwrap_or("None")
                ));
                update_file(src, dest, sftp, session)?;
            }
        }
        Err(e) => match e.code() {
            ErrorCode::SFTP(c) => {
                if c == 2 {
                    sc.run(format!(
                        "Upload: {}",
                        dest.as_path().to_str().unwrap_or("None")
                    ));
                    upload_file(src, dest, session)?;
                } else {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Erreur : Code {} Message {}", c, e.message()),
                    ));
                }
            }
            ErrorCode::Session(_) => return Err(Error::new(ErrorKind::Other, "Erreur inattendue")),
        },
    }
    Ok(())
}

fn sync_folder(
    src: &PathBuf,
    dest: &PathBuf,
    rc: &Arc<Callable<String>>,
    sftp: &Arc<Sftp>,
) -> Result<()> {
    let imeta = gen_file_stat(src);
    match sftp.opendir(dest) {
        Ok(mut dir) => {
            rc.run(format!(
                "recursion {}",
                dest.as_path().to_str().unwrap_or("None")
            ));
            dir.setstat(imeta).unwrap_or(());
        }
        Err(e) => match e.code() {
            ErrorCode::SFTP(c) => {
                if c == 2 {
                    rc.run(format!(
                        "mkdir {}",
                        dest.as_path().to_str().unwrap_or("None")
                    ));
                    sftp.mkdir(dest, imeta.perm.unwrap_or(644) as i32)?;
                    sftp.open(dest)?.setstat(imeta).unwrap_or(());
                } else {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Erreur : Code {} Message {}", c, e.message()),
                    ));
                }
            }
            ErrorCode::Session(_) => return Err(Error::new(ErrorKind::Other, "Erreur Inattendue")),
        },
    }
    Ok(())
}

fn sync_tree(
    src: &PathBuf,
    dest: &PathBuf,
    sc: &Arc<Callable<String>>,
    rc: &Arc<Callable<String>>,
    sftp: &Arc<Sftp>,
    session: &Arc<Session>,
    pool: &Arc<ThreadPool>,
    tx: &SyncSender<ThreadMessage>,
    max_retries: &u32,
) -> Result<()> {
    if src.is_dir() {
        sync_folder(src, dest, rc, sftp)?;
        let entries = fs::read_dir(src)?;
        for e in entries {
            let entry = e?;
            let scc = sc.clone();
            let rcc = rc.clone();
            let sftpc = sftp.clone();
            let sessc = session.clone();
            let poolc = pool.clone();
            let txc = tx.clone();
            pool.install(move || {
                match sync_tree(
                    &src.join(entry.file_name()),
                    &dest.join(entry.file_name()),
                    &scc,
                    &rcc,
                    &sftpc,
                    &sessc,
                    &poolc,
                    &txc,
                    max_retries,
                ) {
                    Ok(()) => (),
                    Err(e) => {
                        tx.send(ThreadMessage::from_error(e)).unwrap();
                    }
                }
            });
        }
    } else if src.is_file() {
        let mut retry_run = true;
        let mut retry_count = 0;

        while retry_run {
            match sync_file(src, dest, sc, sftp, session) {
                Ok(()) => {
                    retry_run = false;
                }
                Err(e) => {
                    retry_count += 1;
                    thread::sleep(Duration::from_secs(1));
                    if &retry_count >= max_retries {
                        return Err(e);
                    }
                }
            }
        }
    }
    Ok(())
}

pub struct SshSync {
    paths: (PathBuf, PathBuf),
    host: Host,
    delete: bool,
    max_retries: u32,
    max_threads: usize,
    rec_cb: Arc<Callable<String>>,
    sync_cb: Arc<Callable<String>>,
}

impl Synchro for SshSync {
    fn sync(&self) -> SynchroResult<SynchroState> {
        if self.host.name().eq("UNUSABLE") {
            return Err(SynchroError::from(Error::new(
                ErrorKind::Other,
                self.host.user(),
            )));
        } else {
            let addr = SocketAddr::from((self.host.ip().octets(), 22));
            let tcp = TcpStream::connect(addr)?;
            let mut sess = Session::new()?;
            sess.set_tcp_stream(tcp);
            sess.handshake()?;
            sess.userauth_password(&self.host.user(), &self.host.password())?;
            if sess.authenticated() {
                let asftp = Arc::new(sess.sftp()?);
                let asess = Arc::new(sess);
                let pool = Arc::new(
                    ThreadPoolBuilder::new()
                        .num_threads(self.max_threads)
                        .build()
                        .unwrap(),
                );
                let (tx, rx) = sync_channel(32);
                let src = PathBuf::from(&self.paths.0);
                let dest = PathBuf::from(&self.paths.1);
                let mr = self.max_retries;
                let del = self.delete;
                let run_error_capture = Arc::new(AtomicBool::new(true));
                let wrac = Arc::downgrade(&run_error_capture);
                let rc = self.rec_cb.clone();
                let sc = self.rec_cb.clone();
                let jh = thread::spawn(move || -> Result<()> {
                    sync_tree(&src, &dest, &sc, &rc, &asftp, &asess, &pool, &tx, &mr)?;
                    if del {
                        cleanup(&src, &dest, &asftp, &pool, &tx)?;
                    }
                    println!("DONE");
                    (*run_error_capture).store(false, Ordering::SeqCst);
                    Ok(())
                });
                let mut errvec = Vec::new();

                while match wrac.upgrade() {
                    Some(working) => (*working).load(Ordering::SeqCst),
                    None => false,
                } {
                    let tm = match rx.recv() {
                        Ok(message) => match message {
                            ThreadMessage::Er(e) => Some(e.0),
                            _ => None,
                        },
                        Err(_) => None, // TODO : Something cleaner here. But to avoid the "closed channel" error at the end of the execution
                    };

                    if tm.is_some() {
                        errvec.push(tm.unwrap());
                    }
                }

                jh.join().unwrap()?;

                for err in errvec {
                    return Err(SynchroError::from(err));
                }
            } else {
                return Err(SynchroError::from(Error::new(
                    ErrorKind::Other,
                    "Connexion SSH non établie",
                )));
            }
        }
        Ok(SynchroState::Good(0))
    }
}

impl SshSync {
    pub fn new(src: &Path, dest: &Path, host: Host, max_retries: u32) -> Self {
        SshSync {
            paths: (src.to_path_buf(), dest.to_path_buf()),
            host,
            delete: false,
            max_retries,
            max_threads: 8,
            sync_cb: Arc::new(Callable::new(Box::new(|_: String| {}))),
            rec_cb: Arc::new(Callable::new(Box::new(|_: String| {}))),
        }
    }

    pub fn host(&self) -> &Host {
        &self.host
    }

    pub fn paths(&self) -> (&PathBuf, &PathBuf) {
        (&self.paths.0, &self.paths.1)
    }

    pub fn delete(&self) -> bool {
        self.delete
    }

    pub fn set_delete(&mut self, d: bool) {
        self.delete = d;
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
}
