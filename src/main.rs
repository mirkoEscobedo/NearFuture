use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || (args.len() == 1 && args[0] == "--version") {
        println!("{}", nf_contract::BUILD_IDENTITY);
        ExitCode::SUCCESS
    } else {
        eprintln!("Usage: near-future [--version]");
        ExitCode::from(2)
    }
}
