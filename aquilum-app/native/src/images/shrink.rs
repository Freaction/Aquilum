use super::decode::Decoded;

fn rgba(px: &[u8]) -> [u8; 4] {
    match *px {
        [l] => [l, l, l, 255],
        [l, a] => [l, l, l, a],
        [r, g, b] => [r, g, b, 255],
        [r, g, b, a, ..] => [r, g, b, a],
        [] => [0; 4],
    }
}

pub struct Shrink {
    to: (u32, u32),
    from_height: u32,
    columns: Vec<u32>,
    sums: Vec<u64>,
    row: u32,
    out: Vec<u8>,
}

impl Shrink {
    pub fn new(from: (u32, u32), to: (u32, u32)) -> Shrink {
        let columns = (0..from.0).map(|x| (u64::from(x) * u64::from(to.0) / u64::from(from.0)) as u32).collect();
        Shrink {
            to,
            from_height: from.1,
            columns,
            sums: vec![0; to.0 as usize * 5],
            row: 0,
            out: Vec::with_capacity(to.0 as usize * to.1 as usize * 4),
        }
    }

    pub fn push(&mut self, row: &[u8], channels: usize) {
        if (self.columns.len() as u32, self.from_height) == self.to {
            self.out.extend(row.chunks_exact(channels).flat_map(rgba));
            return;
        }
        for (px, &column) in row.chunks_exact(channels).zip(&self.columns) {
            let [r, g, b, a] = rgba(px);
            let a = u64::from(a);
            let at = column as usize * 5;
            let sum = &mut self.sums[at..at + 5];
            sum[0] += u64::from(r) * a;
            sum[1] += u64::from(g) * a;
            sum[2] += u64::from(b) * a;
            sum[3] += a;
            sum[4] += 1;
        }
        let line = |y: u32| u64::from(y) * u64::from(self.to.1) / u64::from(self.from_height);
        self.row += 1;
        if line(self.row) != line(self.row - 1) {
            self.flush();
        }
    }

    fn flush(&mut self) {
        for sum in self.sums.chunks_exact_mut(5) {
            let (alpha, count) = (sum[3], sum[4].max(1));
            let pixel = if alpha == 0 {
                [0; 4]
            } else {
                let channel = |s: u64| ((s + alpha / 2) / alpha) as u8;
                [channel(sum[0]), channel(sum[1]), channel(sum[2]), ((alpha + count / 2) / count) as u8]
            };
            self.out.extend_from_slice(&pixel);
            sum.fill(0);
        }
    }

    pub fn finish(self, natural: (u32, u32)) -> Option<Decoded> {
        let (width, height) = self.to;
        (self.out.len() == width as usize * height as usize * 4).then_some(Decoded { width, height, pixels: self.out, natural })
    }
}
