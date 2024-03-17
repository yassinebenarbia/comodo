use std::{
    fs::{File, self}, env, os::unix::net::{UnixStream, UnixListener},
    io::{Write, Read, BufReader, BufRead, SeekFrom, Seek, self, ErrorKind}, path::Path,
    net::{TcpStream, TcpListener},
    time::{Duration, SystemTime, UNIX_EPOCH}, thread, 
};

use config::{Comodoro, Config};
use sysinfo::{Pid, Signal, System};
use notify_rust::{Notification, Timeout};

use clap::{Parser, Subcommand};
use daemonize::Daemonize;
use rev_buf_reader::RevBufReader;
mod config;

const DEFAULT: [u8;1] = [0];
const PAUSE: [u8;1] = [1];
const RESUME: [u8;1] = [2];
const STOP: [u8;1] = [3];
const KILL: [u8;1] = [4];

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
/// Make any Linux distribution repeatable!
pub struct Cli {
    #[command(subcommand)]
    pub state: State,
}

#[derive(Subcommand, Debug)]
pub enum State {
    /// initiate a backgroud daemon if it does not exist
    Init,
    /// Kills the backgroud daemon if it exist
    Kill,
    Pause,
    Start {
        #[arg(short, long, default_value_t = 5)]
        /// Pomodoro duration in seconds
        focus: u64,
        #[arg(short, long, default_value_t = 5)]
        /// Rest duration in seconds
        rest: u64,
        #[arg(short, long, default_value_t = 1)]
        /// Number of cycles
        number: u8,
        #[arg(short, long)]
        /// Path for the config file
        config: String
    },
    Stop,
    Resume,
    Status,
    From,
}

pub fn as_time(seconds: u64) -> String{
    let left_minutes = if seconds / 60 < 10 {
        format!("0{}", seconds / 60)
    }else {
        format!("{}", seconds/60)
    };

    let left_seconds = if seconds % 60 < 10 {
        format!("0{}", seconds % 60)
    }else {
        format!("{}", seconds % 60)
    };

    return format!("{}:{}",left_minutes, left_seconds);
}

fn main() {
    let command = Cli::parse(); 
    let socket_path = "/tmp/comodoro.sock";
    let state_path = "/tmp/state.sock";
    let daemon_stdout = "/tmp/comodoro.out";
    let daemon_stderr = "/tmp/comodoro.err";

    match command.state {
        State::Pause => {
            let mut stream = UnixStream::connect(state_path).unwrap();
            stream.write_all(&PAUSE).unwrap();
        },
        State::Start { focus, rest, number, config } => {
            let cconfig = if !config.is_empty() {
                let content = fs::read_to_string(config).unwrap();
                let config: Config = toml::de::from_str(content.as_str()).unwrap();
                config.comodoro
            }else {
                Comodoro {
                    iterations: number,
                    focus: Duration::from_secs(focus),
                    rest: Duration::from_secs(rest),
                    big_rest: Duration::from_secs(900)
                }
            };

            let mut stream = UnixStream::connect(socket_path).unwrap();
            UnixStream::connect(state_path).unwrap();

            let now = SystemTime::now();
            let since_the_epoch = now.duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let in_ms = since_the_epoch.as_secs();

            stream.write_all(&DEFAULT).unwrap();
            stream.write_all(&cconfig.focus.as_secs().to_be_bytes()).unwrap();
            stream.write_all(&cconfig.rest.as_secs().to_be_bytes()).unwrap();
            stream.write_all(&cconfig.iterations.to_be_bytes()).unwrap();
            stream.write_all(&in_ms.to_be_bytes()).unwrap();
        },
        State::Stop => {
            let mut state_stream = UnixStream::connect(state_path).unwrap();
            state_stream.write_all(&STOP).unwrap();
        },
        State::Resume => {
            let mut state_stream = UnixStream::connect(state_path).unwrap();
            state_stream.write_all(&RESUME).unwrap();
        },
        State::Status => {
            const MAX:u8 = 11;
            let mut i:u8 = 0;

            let thing = TcpListener::bind("127.0.0.1:8080").unwrap();
            thing.set_nonblocking(true).unwrap();

            for stream in thing.incoming() {
                thread::sleep(Duration::from_millis(50));
                match stream {
                    Ok(mut stream) => {
                        let mut body = String::new();
                        stream.read_to_string(&mut body).unwrap();
                        println!("{}", body);
                        break;
                    }
                    Err(_) => {
                        i+=1;
                    }
                }
                if i == MAX {
                    println!("No pomodoro is running!");
                    break;
                }
            }
        },
        State::From => todo!(),
        // TODO: check if daemon is already running
        // TODO: check for necessery file
        State::Init => {
            if Path::new(socket_path).exists() {
                println!("Socket file already exist, trying to removing it...");
                std::fs::remove_file(socket_path).unwrap();
                println!("Socket file removed!");
            }
            if Path::new(state_path).exists() {
                println!("state file already exist, trying to removing it...");
                std::fs::remove_file(state_path).unwrap();
                println!("state file removed!");
            }

            let stdout = File::create(daemon_stdout).unwrap();
            let stderr = File::create(daemon_stderr).unwrap();
            if let Ok(stream) = TcpStream::connect("127.0.0.1:8080") {
                println!("Connected to the server!");
            } else {
                println!("Couldn't connect to server...");
            }

            let daemonize = Daemonize::new()
                .chown_pid_file(true)      
                .working_directory(env::current_dir().unwrap())
                .user("nobody")
                .group("daemon") // Group name
                .umask(0o027)    // Set umask, `0o027` by default.
                .stdout(stdout)
                .stderr(stderr)
                .privileged_action(|| "Executed before drop privileges");

            println!("Starting the daemon...");
            match daemonize.start() {
                Ok(thing) => println!("Success, daemonized {}", thing),
                Err(e) => eprintln!("Error, {}", e),
            }

            #[allow(unused_assignments)]
            let mut on_focusing = false;
            let mut on_focusing_ = false;
            let socket_stream = UnixListener::bind(socket_path).unwrap();
            let state_stream = UnixListener::bind(state_path).unwrap();

            state_stream.set_nonblocking(true).unwrap();

            for stream in socket_stream.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let mut _kill_buffer = [0];
                        let mut _focus_buffer = [0; 8];
                        let mut _rest_buffer = [0; 8];
                        let mut _elapsed_buffer = [0; 8];
                        let mut _number_buffer = [0];
                        let mut pausing = false;
                        let mut stopping = false;

                        stream.read(&mut _kill_buffer).unwrap_or(0 as usize);
                        if _kill_buffer == KILL {
                            let s = System::new_all();
                            for process in s.processes_by_name("comodoro") {
                                process.kill();
                            }
                            return;
                        }

                        stream.read(&mut _focus_buffer).unwrap_or(0 as usize);
                        stream.read(&mut _rest_buffer).unwrap_or(0 as usize);
                        stream.read(&mut _number_buffer).unwrap_or(0 as usize);
                        stream.read(&mut _elapsed_buffer).unwrap_or(0 as usize);

                        let focus = u64::from_be_bytes(_focus_buffer);
                        let rest = u64::from_be_bytes(_rest_buffer);
                        let elapsed = u64::from_be_bytes(_elapsed_buffer);
                        let number = u8::from_be_bytes(_number_buffer);
                        let duty_duration =focus.wrapping_add(rest);

                        let mut now = SystemTime::now();
                        let mut since_the_epoch = now.duration_since(UNIX_EPOCH)
                            .expect("Time went backwards");
                        let mut now_in_secs = since_the_epoch.as_secs();

                        let mut paused_duration = Duration::from_secs(0).as_secs();
                        let mut _now = SystemTime::now();
                        let mut __now = SystemTime::now();

                        let mut time_since_started = now_in_secs - elapsed;
                        let mut n = time_since_started / duty_duration;

                        while n < number as u64 {
                            thread::sleep(Duration::from_millis(500));
                            // what am I trying to calculate
                            // 1) did we finished our duty?
                            // 2) what is the current countdown time?
                            // 3) on what state are we (focusing vs rest)

                            for action_buffer in state_stream.incoming() {
                                match action_buffer {
                                    Ok(mut stream) => {
                                        let mut action = DEFAULT;
                                        stream.read(&mut action).unwrap();

                                        // checking for signals
                                        if action == PAUSE {
                                            _now = SystemTime::now();
                                            pausing = true;
                                            println!("pause recived");
                                        }else if action == RESUME {
                                            __now = SystemTime::now();
                                            paused_duration = paused_duration
                                                .checked_add(
                                                    __now.duration_since(_now)
                                                    .unwrap_or(Duration::ZERO)
                                                    .as_secs()
                                                    ).unwrap_or(0);
                                            pausing = false;
                                        }else if action == STOP {
                                            stopping = true;
                                        }
                                        break;
                                    },
                                    Err(_) => { break; },
                                }
                            }

                            let t = time_since_started % duty_duration;
                            let focusing = focus.checked_sub(t).unwrap_or(0) > 0;
                            let focusing_duration = if focusing {
                                t
                            }else {
                                0
                            };
                            let rest_duration= t.checked_sub(focus).unwrap_or(0);

                            if pausing {
                                if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
                                    stream.write_all(
                                        format!("stateus: Pause\r\niteration: {}/{}\r\nfocus: {}/{}\r\nrest: {}/{}\r\n",
                                                number, n,
                                                as_time(focusing_duration), as_time(focus),
                                                as_time(rest_duration), as_time(rest)).as_bytes()).unwrap();
                                }

                                continue;
                            } else if stopping{
                                break;
                            }


                            if focusing {
                                if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
                                    stream.write_all(
                                        format!("stateus: focusing\r\niteration: {}/{}\r\nfocus: {}/{}\r\nrest: {}/{}\r\n",
                                                number, n,
                                                as_time(focusing_duration), as_time(focus),
                                                as_time(rest_duration), as_time(rest)).as_bytes()).unwrap();
                                }

                                on_focusing = true;

                                if on_focusing != on_focusing_ {
                                    Notification::new()
                                        .summary("comodoro:pomodoro")
                                        .body(&format!("Start of Pomodoro {}", n + 1))
                                        .appname("comodoro")
                                        .timeout(Timeout::from(Duration::from_secs(2)))
                                        .show().unwrap();
                                }
                                on_focusing_ = true;
                            } else {
                                if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
                                    println!("Connected to the server!");
                                    stream.write_all(
                                        format!("stateus: resting\r\niteration: {}/{}\r\nfocus: {}/{}\r\nrest: {}/{}\r\n",
                                                number, n,
                                                as_time(focusing_duration), as_time(focus),
                                                as_time(rest_duration), as_time(rest)).as_bytes()).unwrap();
                                }

                                on_focusing = false;

                                if on_focusing != on_focusing_ {
                                    Notification::new()
                                        .summary("comodoro:pomodoro")
                                        .body(&format!("Start of Rest {}", n + 1))
                                        .appname("comodoro")
                                        .timeout(Timeout::from(Duration::from_secs(2)))
                                        .show().unwrap();
                                }
                                on_focusing_ = false;
                            }

                            now = SystemTime::now();
                            since_the_epoch = now.duration_since(UNIX_EPOCH)
                                .expect("Time went backwards");
                            now_in_secs = since_the_epoch.as_secs();

                            time_since_started = now_in_secs - elapsed - paused_duration;
                            n = time_since_started / duty_duration;
                        }
                        Notification::new()
                            .summary("comodoro:pomodoro")
                            .body(&format!("End of Session!"))
                            .appname("comodoro")
                            .timeout(Timeout::from(Duration::from_secs(2)))
                            .show().unwrap();
                    },
                    Err(_) => {
                    },
                }
            }
        },
        State::Kill => {
            let mut stream = UnixStream::connect(socket_path).unwrap();
            stream.write_all(&KILL).unwrap();
        },
    }
}
