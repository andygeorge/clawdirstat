use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "clawdirstat", about = "Terminal disk usage visualizer")]
pub struct Args {
    /// Directory to scan (defaults to current directory)
    pub dir: Option<PathBuf>,

    /// Limit number of top-level entries displayed
    #[arg(short = 'n', long)]
    pub count: Option<usize>,
}

pub fn parse() -> Args {
    Args::parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Args {
        Args::parse_from(std::iter::once("clawdirstat").chain(args.iter().copied()))
    }

    #[test]
    fn test_default_no_args() {
        let args = parse_args(&[]);
        assert!(args.dir.is_none());
        assert!(args.count.is_none());
    }

    #[test]
    fn test_explicit_dir() {
        let args = parse_args(&["/tmp"]);
        assert_eq!(args.dir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_n_flag() {
        let args = parse_args(&["-n", "5"]);
        assert_eq!(args.count, Some(5));
    }

    #[test]
    fn test_dir_and_n_flag() {
        let args = parse_args(&["/home", "-n", "10"]);
        assert_eq!(args.dir, Some(PathBuf::from("/home")));
        assert_eq!(args.count, Some(10));
    }
}
