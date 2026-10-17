use crate::files::error::FileCommandError;
use crate::text::content::{Content, Splice};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub const EXTENSION: &str = ".delta";
const MAGIC: &str = "aquilum-delta 1";
const NO_BASE: &str = "-";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub from: usize,
    pub to: usize,
    pub len: usize,
}

impl Region {
    fn grows(self) -> isize {
        self.len as isize - (self.to - self.from) as isize
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delta {
    pub base: Option<String>,
    pub hash: String,
    pub len: usize,
    pub regions: Vec<Region>,
    pub body: u64,
}

impl Delta {
    pub fn inserted(&self) -> usize {
        self.regions.iter().map(|region| region.len).sum()
    }
}

pub fn starts(regions: &[Region]) -> Vec<usize> {
    let mut shift = 0isize;
    regions
        .iter()
        .map(|region| {
            let at = (region.from as isize + shift) as usize;
            shift += region.grows();
            at
        })
        .collect()
}

pub fn whole(len: usize) -> Vec<Region> {
    vec![Region { from: 0, to: 0, len }]
}

pub fn single(splice: Splice) -> Vec<Region> {
    vec![Region { from: splice.at, to: splice.at + splice.removed, len: splice.inserted }]
}

pub fn compose(regions: &[Region], splice: Splice) -> Vec<Region> {
    let at = starts(regions);
    let (s, e) = (splice.at, splice.at + splice.removed);
    let first = (0..regions.len()).find(|&i| at[i] + regions[i].len >= s).unwrap_or(regions.len());
    let last = (first..regions.len()).take_while(|&i| at[i] <= e).last();
    let before: isize = regions[..first].iter().map(|region| region.grows()).sum();
    let (vs, from) = match last {
        Some(_) if at[first] <= s => (at[first], regions[first].from),
        _ => (s, (s as isize - before) as usize),
    };
    let (ve, to, after) = match last {
        Some(last) => {
            let through = before + regions[first..=last].iter().map(|region| region.grows()).sum::<isize>();
            let end = at[last] + regions[last].len;
            if end >= e {
                (end, regions[last].to, last + 1)
            } else {
                (e, (e as isize - through) as usize, last + 1)
            }
        }
        None => (e, (e as isize - before) as usize, first),
    };
    let len = ve - vs - splice.removed + splice.inserted;
    let mut next = regions[..first].to_vec();
    if from != to || len != 0 {
        next.push(Region { from, to, len });
    }
    next.extend_from_slice(&regions[after..]);
    next
}

pub fn read(path: &Path) -> io::Result<Delta> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = String::new();
    let mut body = 0u64;
    let mut next = |reader: &mut BufReader<File>, line: &mut String| -> io::Result<()> {
        line.clear();
        body += reader.read_line(line)? as u64;
        Ok(())
    };
    let broken = || io::Error::new(io::ErrorKind::InvalidData, "повреждённая версия");
    next(&mut reader, &mut line)?;
    if line.trim_end() != MAGIC {
        return Err(broken());
    }
    let mut field = |reader: &mut BufReader<File>, name: &str| -> io::Result<String> {
        next(reader, &mut line)?;
        line.trim_end().strip_prefix(name).and_then(|rest| rest.strip_prefix(' ')).map(str::to_owned).ok_or_else(broken)
    };
    let base = Some(field(&mut reader, "base")?).filter(|base| base != NO_BASE);
    let hash = field(&mut reader, "hash")?;
    let len = field(&mut reader, "len")?.parse().map_err(|_| broken())?;
    let mut regions = Vec::new();
    loop {
        next(&mut reader, &mut line)?;
        let row = line.trim_end();
        if row.is_empty() {
            break;
        }
        let mut numbers = row.split(' ').map(str::parse::<usize>);
        let mut number = || numbers.next().and_then(Result::ok).ok_or_else(broken);
        regions.push(Region { from: number()?, to: number()?, len: number()? });
    }
    Ok(Delta { base, hash, len, regions, body })
}

pub fn write(path: &Path, base: Option<&str>, hash: &str, regions: &[Region], text: &dyn Content) -> Result<(), FileCommandError> {
    let mut header = format!("{MAGIC}\nbase {}\nhash {hash}\nlen {}\n", base.unwrap_or(NO_BASE), text.byte_len());
    for region in regions {
        header.push_str(&format!("{} {} {}\n", region.from, region.to, region.len));
    }
    header.push('\n');
    crate::files::document::atomic(path, None, |out| {
        out.put(&header)?;
        regions.iter().zip(starts(regions)).try_for_each(|(region, at)| out.put_range(text, at..at + region.len))
    })
    .map(drop)
}

#[cfg(test)]
#[path = "delta_tests.rs"]
mod tests;
