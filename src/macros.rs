use std::sync::atomic::AtomicU8;

pub static VERBOSITY: AtomicU8 = AtomicU8::new(3); // 3 = Normal, 2 = Debug, 1 = Verbose, 0 = NoConsole

#[macro_export]
macro_rules! dprintln {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 2) {
            print!("[{}] ", chrono::Local::now().format("%H:%M:%S"));
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! vprintln {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 1) {
            print!("[{}] ", chrono::Local::now().format("%H:%M:%S"));
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! derrprintln {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 2) {
            eprint!("[{}] ", chrono::Local::now().format("%H:%M:%S"));
            eprintln!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! verrprintln {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 1) {
            eprint!("[{}] ", chrono::Local::now().format("%H:%M:%S"));
            eprintln!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! dprint {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 2) {
            print!($($arg)*);
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
    };
}

#[macro_export]
macro_rules! vprint {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 1) {
            print!($($arg)*);
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
    };
}

#[macro_export]
macro_rules! derrprint {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 2) {
            eprint!($($arg)*);
            use std::io::Write;
            let _ = std::io::stderr().flush();
        }
    };
}

#[macro_export]
macro_rules! verrprint {
    ($($arg:tt)*) => {
        if matches!($crate::macros::VERBOSITY.load(std::sync::atomic::Ordering::Relaxed), 1) {
            eprint!($($arg)*);
            use std::io::Write;
            let _ = std::io::stderr().flush();
        }
    };
}

#[macro_export]
macro_rules! set_verbosity {
    ($level:expr) => {
        $crate::macros::VERBOSITY.store($level, std::sync::atomic::Ordering::Relaxed);
    };
}