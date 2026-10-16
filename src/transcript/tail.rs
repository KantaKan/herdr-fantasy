//! Follow a growing JSONL file by byte offset (SPEC §11). Read-only, never locks.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// How much history to read on first sight (enough for the latest usage entry).
const HISTORY_BYTES: u64 = 64 * 1024;

pub struct Tail {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
}

impl Tail {
    /// Start following `path`. Returns the tail plus the complete lines of the last
    /// 64 KB — history to learn the current state from, not to animate.
    pub fn open(path: &Path) -> std::io::Result<(Tail, Vec<String>)> {
        let len = std::fs::metadata(path)?.len();
        let mut t = Tail {
            path: path.to_path_buf(),
            offset: len.saturating_sub(HISTORY_BYTES),
            partial: Vec::new(),
        };
        let starts_mid_line = t.offset > 0;
        let mut history = t.poll()?;
        if starts_mid_line && !history.is_empty() {
            history.remove(0); // we probably landed inside a line
        }
        Ok((t, history))
    }

    /// Complete lines appended since the last call. A file that shrank was replaced or
    /// truncated: start over from its beginning.
    pub fn poll(&mut self) -> std::io::Result<Vec<String>> {
        let len = std::fs::metadata(&self.path)?.len();
        if len < self.offset {
            self.offset = 0;
            self.partial.clear();
        }
        if len == self.offset {
            return Ok(Vec::new());
        }
        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::with_capacity((len - self.offset) as usize);
        f.take(len - self.offset).read_to_end(&mut buf)?;
        self.offset += buf.len() as u64;
        self.partial.extend_from_slice(&buf);
        let Some(last_nl) = self.partial.iter().rposition(|&b| b == b'\n') else {
            return Ok(Vec::new());
        };
        let rest = self.partial.split_off(last_nl + 1);
        let done = std::mem::replace(&mut self.partial, rest);
        Ok(String::from_utf8_lossy(&done)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(String::from)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hf-tail-{name}-{}.jsonl", std::process::id()))
    }

    fn append(p: &Path, s: &str) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .unwrap();
        f.write_all(s.as_bytes()).unwrap();
    }

    #[test]
    fn history_then_only_new_lines_with_split_writes() {
        let p = tmp("split");
        std::fs::write(&p, "old1\nold2\n").unwrap();
        let (mut t, history) = Tail::open(&p).unwrap();
        assert_eq!(history, ["old1", "old2"]);
        assert!(t.poll().unwrap().is_empty(), "no replay");
        append(&p, "new1\nhal");
        assert_eq!(t.poll().unwrap(), ["new1"]);
        append(&p, "f\n");
        assert_eq!(t.poll().unwrap(), ["half"]);
    }

    #[test]
    fn truncation_starts_over() {
        let p = tmp("trunc");
        std::fs::write(&p, "a long line here\nanother\n").unwrap();
        let (mut t, _) = Tail::open(&p).unwrap();
        std::fs::write(&p, "fresh\n").unwrap();
        assert_eq!(t.poll().unwrap(), ["fresh"]);
    }

    #[test]
    fn big_file_history_skips_the_cut_line() {
        let p = tmp("big");
        let line = format!("{}\n", "x".repeat(1000));
        std::fs::write(&p, line.repeat(100)).unwrap();
        append(&p, "last\n");
        let (_, history) = Tail::open(&p).unwrap();
        assert_eq!(history.last().unwrap(), "last");
        assert!(history.iter().all(|l| l == "last" || l.len() == 1000));
    }
}
