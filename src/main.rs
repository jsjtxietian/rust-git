use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;
use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Doc comment
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,
        object_hash: String,
    },
}

enum Kind {
    Blob,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
            anyhow::ensure!(pretty_print, "mode must be gived without -p");
            let mut f = std::fs::File::open(format!(
                ".git/objects/{}/{}",
                &object_hash[..2],
                &object_hash[2..]
            ))
            .context("open in ./git/objects")?;
            let z = ZlibDecoder::new(f);
            let mut z = BufReader::new(z);
            let mut buf = Vec::new();
            z.read_until(0, &mut buf)
                .context("Read header from .git/objects")?;
            let header =
                CStr::from_bytes_with_nul(&buf).expect("Exactly one nul and it's at the end");
            let header = header
                .to_str()
                .context(".git/objects file header is not valid utf8")?;
            let Some((kind, size)) = header.split_once(' ') else {
                anyhow::bail!(
                    ".git/objects file header does not start with a known type '{header}'"
                );
            };
            let kind = match kind {
                "blob" => Kind::Blob,
                _ => anyhow::bail!("We do not know how to print a {kind}"),
            };
            let size = size
                .parse::<u64>()
                .context(".git/objects file header has invalid size: {size}")?;

            let mut z = z.take(size);
            match kind {
                Kind::Blob => {
                    let mut stdout = std::io::stdout().lock();
                    let n = std::io::copy(&mut z, &mut stdout)
                        .context("Write .git/objects to stdout")?;
                    anyhow::ensure!(
                        n == size.try_into().unwrap(),
                        ".git/objects file {n} was not the expected size {size}"
                    );
                }
            }
        }
    }

    Ok(())
}
