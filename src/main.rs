use std::{
    fs::File, env, os::unix::net::{UnixStream, UnixListener},
    io::{Write, Read}, path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH}
};
use notify_rust::{Notification, Timeout};

use clap::{Parser, Subcommand};
use daemonize::Daemonize;

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
    },
    Stop,
    Resume,
    Status,
    From,
}

fn main() {
    let command = Cli::parse(); 
    let socket_path = "/tmp/daemon.sock";
    let state_path = "/tmp/state.sock";
    let report_state_path = "/tmp/repst.sock";

    match command.state {
        State::Pause => {
            let mut stream = UnixStream::connect(state_path).unwrap();
            stream.write_all(&PAUSE).unwrap();
        },
        State::Start { focus, rest, number } => {
            let mut stream = UnixStream::connect(socket_path).unwrap();
            UnixStream::connect(state_path).unwrap();

            let now = SystemTime::now();
            let since_the_epoch = now.duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let in_ms = since_the_epoch.as_secs();

            stream.write_all(&DEFAULT).unwrap();
            stream.write_all(&focus.to_be_bytes()).unwrap();
            stream.write_all(&rest.to_be_bytes()).unwrap();
            stream.write_all(&number.to_be_bytes()).unwrap();
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
            let repst = UnixListener::bind(report_state_path).unwrap();
            repst.set_nonblocking(true).unwrap();

            // checks for data, if exist it shows it, else do nothing
            for stream in repst.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let mut _kill_buffer = [0];
                        let mut _focus_buffer = [0; 8];
                        let mut _rest_buffer = [0; 8];
                        let mut _elapsed_buffer = [0; 8];
                        let mut _number_buffer = [0];

                        stream.read(&mut _kill_buffer).unwrap_or(0 as usize);
                        stream.read(&mut _focus_buffer).unwrap_or(0 as usize);
                        stream.read(&mut _rest_buffer).unwrap_or(0 as usize);
                        stream.read(&mut _number_buffer).unwrap_or(0 as usize);
                        stream.read(&mut _elapsed_buffer).unwrap_or(0 as usize);

                        let focus = u64::from_be_bytes(_focus_buffer);
                        let rest = u64::from_be_bytes(_rest_buffer);
                        let elapsed = u64::from_be_bytes(_elapsed_buffer);
                        let number = u8::from_be_bytes(_number_buffer);

                        println!("focus: {:#?}", focus);
                        println!("rest: {:#?}", rest);
                        println!("number: {:#?}", number);
                        println!("elapsed: {:#?}", elapsed);
                        break;
                    }
                    Err(_) => {
                        println!("Nothing to show for now!");
                        break;
                    }
                }
            }
        },
        State::From => todo!(),
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
            if Path::new(report_state_path).exists() {
                println!("Report state file already exist, trying to removing it...");
                std::fs::remove_file(report_state_path).unwrap();
                println!("Report state file removed!");
            }

            let stdout = File::create("/tmp/daemon.out").unwrap();
            let stderr = File::create("/tmp/daemon.err").unwrap();
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

                        let mut repost = UnixStream::connect(report_state_path).unwrap();
                        repost.write_all(&DEFAULT).unwrap();
                        repost.write_all(&_focus_buffer).unwrap();
                        repost.write_all(&_rest_buffer).unwrap();
                        repost.write_all(&_elapsed_buffer).unwrap();
                        repost.write_all(&_number_buffer).unwrap();

                        println!("focus: {:#?}", focus);
                        println!("rest: {:#?}", rest);
                        println!("number: {:#?}", number);
                        println!("elapsed: {:#?}", elapsed);

                        let mut now = SystemTime::now();
                        let mut since_the_epoch = now.duration_since(UNIX_EPOCH)
                            .expect("Time went backwards");
                        let mut now_in_secs = since_the_epoch.as_secs();

                        let mut paused_duration = Duration::from_secs(0).as_secs();
                        let mut _now = SystemTime::now();
                        let mut __now = SystemTime::now();

                        let mut diff = now_in_secs - elapsed;
                        let mut n = diff / duty_duration;

                        while n < number as u64 {
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
                                            println!("pause recivedd")
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

                            if pausing {
                                continue;
                            } else if stopping{
                                break;
                            }

                            let t = diff % duty_duration;
                            let focusing = focus.checked_sub(t).unwrap_or(0) > 0;

                            if focusing {
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

                            diff = now_in_secs - elapsed - paused_duration;
                            n = diff / duty_duration;
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
