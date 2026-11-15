//! Streaming declaration identities without copying the complete OBJ body.
use std::borrow::Cow;
use std::io::{self, Read};

pub(super) struct ObjSource<'a> {
    lines: std::str::SplitInclusive<'a, char>,
    pending: Cow<'a, [u8]>,
    offset: usize,
    pub part_names: Vec<&'a str>,
}

impl<'a> ObjSource<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { lines: text.split_inclusive('\n'), pending: Cow::Borrowed(b"mtllib texturecraft-usemtl.mtl\n"), offset: 0, part_names: Vec::new() }
    }
}

impl Read for ObjSource<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        while self.offset == self.pending.len() {
            let Some(line) = self.lines.next() else { return Ok(0) };
            let trimmed = line.trim();
            self.pending = if matches!(trimmed.split_whitespace().next(), Some("o" | "g")) {
                let key = format!("o {}\n", self.part_names.len());
                self.part_names.push(trimmed.get(1..).unwrap_or_default().trim());
                Cow::Owned(key.into_bytes())
            } else {
                Cow::Borrowed(line.as_bytes())
            };
            self.offset = 0;
        }
        let remaining = self.pending.get(self.offset..).unwrap_or_default();
        let count = out.len().min(remaining.len());
        out.iter_mut().zip(remaining.iter()).take(count).for_each(|(dst, src)| *dst = *src);
        self.offset += count;
        Ok(count)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_reads_preserve_body_and_distinct_declarations() {
        let text = "# café\r\n o same\r\nv 0 0 0\r\ng\tsame\nusemtl gold\nf 1 2 3";
        let mut source = ObjSource::new(text);
        assert_eq!(source.read(&mut []).unwrap(), 0);
        let mut actual = Vec::new();
        let mut byte = [0];
        while source.read(&mut byte).unwrap() != 0 {
            actual.push(byte[0]);
        }
        assert_eq!(String::from_utf8(actual).unwrap(), "mtllib texturecraft-usemtl.mtl\n# café\r\no 0\nv 0 0 0\r\no 1\nusemtl gold\nf 1 2 3");
        assert_eq!(source.part_names, ["same", "same"]);
        assert_eq!(source.read(&mut byte).unwrap(), 0);
    }
}
