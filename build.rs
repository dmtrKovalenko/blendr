use clap::CommandFactory;

#[path = "src/cli_args.rs"]
mod cli_args;

fn main() -> std::io::Result<()> {
    let out_dir =
        // This is a hack to get the root of the project, since OUT_DIR is relative to the build
        // Super simplifies CI and locating man files after output.
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or(std::io::ErrorKind::NotFound)?)
            .join("../../..");

    let cmd = cli_args::Args::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(out_dir.join("blendr.1"), buffer)?;

    Ok(())
}
