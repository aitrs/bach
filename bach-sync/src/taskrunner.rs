use crate::deltasync::*;
use crate::scriptrunner::ScriptRunner;
use crate::synchrotrait::{Synchro, SynchroError, SynchroResult, SynchroState};
use crate::task::*;
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;

#[derive(Debug, Clone)]
pub struct TaskError {
    message: String,
    code: u64,
}

impl Error for TaskError {}

impl Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Task Erroe => Code {} : {}",
            self.code, self.message
        ))
    }
}

impl TaskError {
    pub fn new(message: &str, code: u64) -> Self {
        TaskError {
            message: message.to_string(),
            code,
        }
    }
}

pub type TaskResult<T> = Result<T, TaskError>;

pub type TaskMessage = (SynchroState, std::thread::ThreadId);

type DImpl = dyn crate::synchrotrait::Synchro;

pub struct TaskRunner {
    prelude_scripts: Vec<TaskCustomScript>,
    syncrunner: Option<Box<DImpl>>,
    post_scripts: Vec<TaskCustomScript>,
    ctx: Sender<TaskMessage>,
    whence: TaskCron,
}

#[macro_export]
macro_rules! mk_prep_runner {
    ($x:ty, $y:ty) => {{
        |el: &$x| -> VirtSyncResult<$y> {
            let mut runner = <$y>::new();
            runner
                .connect(&el.connection)?
                .set_domain(&el.domain)?
                .set_snapshot_count(el.snap_count)
                .set_snapshot_prefix(&el.snap_name)
                .set_paths((PathBuf::from(&el.src), PathBuf::from(&el.dest)));
            for d in &el.disks {
                runner.add_disk(&d.device, &Path::new(&d.file));
            }
            Ok(runner)
        }
    }};
}

impl TaskRunner {
    pub fn new(task: Task, ctx: Sender<TaskMessage>) -> Self {
        match task {
            Task::FileSystemDeltaSyncro(e) => {
                let runner = fssync::FsSync::new(&Path::new(&e.src), &Path::new(&e.dest));
                TaskRunner {
                    prelude_scripts: e.pre_scripts,
                    syncrunner: Some(Box::new(runner)),
                    post_scripts: e.post_scripts,
                    ctx,
                    whence: e.whence,
                }
            }
            Task::SshDeltaSynchro(e) => TaskRunner {
                prelude_scripts: e.pre_scripts,
                syncrunner: Some(Box::new(sshsync::SshSync::new(
                    &Path::new(&e.src),
                    &Path::new(&e.dest),
                    e.host,
                    e.max_retries,
                ))),
                post_scripts: e.post_scripts,
                ctx,
                whence: e.whence,
            },
            #[cfg(feature = "virt-support")]
            Task::FileSystemVirtSynchro(e) => {
                let mut ret = TaskRunner {
                    prelude_scripts: Vec::new(),
                    syncrunner: None,
                    post_scripts: Vec::new(),
                    ctx,
                    whence: e.whence,
                };
                let prep_runner = mk_prep_runner!(TaskFsVirtSync, virtfssync::VirtFsSync);
                match prep_runner(&e) {
                    Ok(vrunner) => {
                        ret.prelude_scripts = e.pre_scripts;
                        ret.syncrunner = Some(Box::new(vrunner));
                        ret.post_scripts = e.post_scripts;
                    }
                    Err(e) => {
                        ret.creation_error = Some(TaskError::from(e));
                    }
                }

                ret
            }
            #[cfg(feature = "virt-support")]
            Task::SshVirtSynchro(e) => {
                let mut ret = TaskRunner {
                    prelude_scripts: Vec::new(),
                    syncrunner: None,
                    post_scripts: Vec::new(),
                    ctx,
                    whence: e.whence,
                };
                let prep_runner = mk_prep_runner!(TaskSshVirtSync, virtsshsync::VirtSshSync);
                match prep_runner(&e) {
                    Ok(vrunner) => {
                        ret.prelude_scripts = e.pre_scripts;
                        ret.syncrunner = Some(Box::new(vrunner));
                        ret.post_scripts = e.post_scripts;
                    }
                    Err(e) => {
                        ret.creation_error = Some(TaskError::from(e));
                    }
                }

                ret
            }
            Task::CustomSynchro(e) => TaskRunner {
                prelude_scripts: e.pre_scripts,
                syncrunner: Some(Box::new(ScriptRunner(e.main_script))),
                post_scripts: e.post_scripts,
                ctx,
                whence: e.whence,
            },
        }
    }

    pub fn run(&self) -> SynchroResult<()> {
        let th_id = thread::current().id();
        for s in &self.prelude_scripts {
            let runner = ScriptRunner(s.clone());
            self.ctx.send((runner.sync()?, th_id))?;
        }
        match &self.syncrunner {
            Some(runner) => {
                self.ctx.send((runner.sync()?, th_id))?;
            }
            None => {
                return Err(SynchroError::new(
                    "Aucune procédure de synchro n'est définie".to_string(),
                    144,
                ))
            }
        }
        for s in &self.post_scripts {
            let runner = ScriptRunner(s.clone());
            self.ctx.send((runner.sync()?, th_id))?;
        }

        Ok(())
    }

    pub fn whence(&self) -> u64 {
        self.whence.get_whence()
    }
}
