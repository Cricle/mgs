use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: mgs-ssh <fingerprint>");
        process::exit(1);
    }

    let fingerprint = &args[1];

    if let Err(e) = mgs::ssh::handle_ssh_command(fingerprint) {
        eprintln!("mgs-ssh: {:#}", e);
        process::exit(1);
    }
}
