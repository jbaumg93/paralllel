use clap::Parser;
use concurrent_queue::ConcurrentQueue;
use indicatif::{ProgressBar, ProgressStyle};
use signal_hook::{consts::SIGINT, consts::SIGUSR1, consts::SIGUSR2, iterator::Signals};
use std::env;
use std::fs;
use std::io;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

#[derive(PartialEq)]
enum ThreadResponse {
    PleaseCreateNewThread,
    JobDone,
    NoNewThreads,
    PleaseReduceThreads,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the input file (if omitted, stdin is used)
    #[arg(short, long)]
    input_file: Option<String>,

    /// Number of threads initially created
    #[arg(short, long, default_value_t = 1)]
    threads: u32,

    /// Delimiter for input values
    #[arg(short, long, default_value_t = String::from("\t"))]
    delimiter: String,

    /// Does not show the progress, prints stdout and stderr of the subprocesses
    #[arg(short, long, default_value_t = false)]
    show_output_instead_of_status: bool,

    /// Execute this command (in parallel), replaces {N} with the Nth value (index starts at 0)
    #[arg()]
    command: String,
}

fn create_thread(
    tx: &Sender<ThreadResponse>,
    q: &Arc<Mutex<ConcurrentQueue<String>>>,
    show_output_instead_of_status: bool,
) -> JoinHandle<()> {
    let tx_thread = tx.clone();
    let q_thread = Arc::clone(q);
    thread::spawn(move || {
        let command = match q_thread.lock().expect("could not get lock").pop() {
            Ok(command) => command,
            Err(_) => {
                tx_thread.send(ThreadResponse::NoNewThreads).unwrap();
                return;
            }
        };

        let shell: String = env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"));
        if !show_output_instead_of_status {
            Command::new(shell)
                .arg("-c")
                .arg(command)
                .process_group(0)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("failed to execute process");
        } else {
            Command::new(shell)
                .arg("-c")
                .arg(command)
                .process_group(0)
                .status()
                .expect("failed to execute process");
        }

        tx_thread.send(ThreadResponse::JobDone).unwrap();
    })
}

fn main() {
    let mut nthreads: u32;
    let mut too_many_threads = 0;
    let q = ConcurrentQueue::unbounded();

    let args = Args::parse();
    let input_data = match args.input_file {
        Some(f) => fs::read_to_string(f).expect("could not read the input file"),
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).unwrap();
            buffer
        }
    };
    nthreads = args.threads;

    for line in input_data.lines() {
        let mut command = args.command.clone();
        for (n, value) in line.split(&args.delimiter).enumerate() {
            command = command.replace(&format!("{{{n}}}"), value);
        }
        q.push(command).unwrap();
    }

    let bar = ProgressBar::new(q.len().try_into().unwrap());
    if !args.show_output_instead_of_status {
        bar.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise} {eta_precise}] {wide_bar:0.red/blue} {pos:>7}/{len:7}",
            )
            .unwrap(),
        );
        bar.inc(0);
    }
    let q = Arc::new(Mutex::new(q));

    let (tx, rx): (Sender<ThreadResponse>, Receiver<ThreadResponse>) = mpsc::channel();
    let mut children = Vec::new();

    let mut signals = Signals::new([SIGINT, SIGUSR1, SIGUSR2]).unwrap();
    let tx_thread = tx.clone();
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => tx_thread.send(ThreadResponse::NoNewThreads).unwrap(),
                SIGUSR1 => tx_thread
                    .send(ThreadResponse::PleaseCreateNewThread)
                    .unwrap(),
                SIGUSR2 => tx_thread.send(ThreadResponse::PleaseReduceThreads).unwrap(),
                _ => panic!(),
            }
        }
    });

    for _ in 0..nthreads {
        children.push(create_thread(&tx, &q, args.show_output_instead_of_status));
    }

    loop {
        match rx.recv().expect("could not get received value") {
            r @ (ThreadResponse::PleaseCreateNewThread | ThreadResponse::JobDone) => {
                if r == ThreadResponse::JobDone && !args.show_output_instead_of_status {
                    bar.inc(1);
                }

                if too_many_threads > 0 {
                    too_many_threads -= 1;
                    nthreads -= 1;
                } else {
                    if r == ThreadResponse::PleaseCreateNewThread {
                        nthreads += 1;
                    }
                    children.push(create_thread(&tx, &q, args.show_output_instead_of_status));
                }
            }
            ThreadResponse::PleaseReduceThreads => {
                if too_many_threads < nthreads {
                    too_many_threads += 1;
                }
            }
            ThreadResponse::NoNewThreads => break,
        }
    }

    for child in children {
        child.join().expect("oops! the child thread panicked");
        if !args.show_output_instead_of_status && bar.length().unwrap() <= bar.position() {
            bar.inc(1);
        }
    }
}
