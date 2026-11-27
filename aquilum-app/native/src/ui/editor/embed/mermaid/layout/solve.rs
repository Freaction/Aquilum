use super::net::{Kind, Net};
use super::place::{Gaps, gap};

pub(super) fn spaces(net: &Net, layer: &[usize], gaps: &Gaps) -> Vec<f64> {
    let mut out: Vec<f64> = layer.windows(2).map(|w| gap(net, w[0], w[1], gaps)).collect();
    let dummy = |k: usize| net.items[layer[k]].kind == Kind::Dummy;
    for k in (0..layer.len()).filter(|&k| !dummy(k)) {
        let mut sum = 0.0;
        for j in k + 1..layer.len() {
            sum += out[j - 1];
            let need = gap(net, layer[k], layer[j], gaps);
            if need > sum {
                out[k] += need - sum;
                sum = need;
            }
            if !dummy(j) {
                break;
            }
        }
        let mut sum = 0.0;
        for j in (0..k).rev() {
            sum += out[j];
            let need = gap(net, layer[j], layer[k], gaps);
            if need > sum {
                out[k - 1] += need - sum;
                sum = need;
            }
            if !dummy(j) {
                break;
            }
        }
    }
    out
}

pub(super) fn solve(targets: &[(f64, f64)], gaps: &[f64]) -> Vec<f64> {
    let mut offset = vec![0.0; targets.len()];
    for i in 1..targets.len() {
        offset[i] = offset[i - 1] + gaps[i - 1];
    }
    let mut blocks: Vec<(f64, f64, usize)> = Vec::new();
    for (&(t, w), o) in targets.iter().zip(&offset) {
        let w = w.max(1e-3);
        blocks.push((w, w * (t - o), 1));
        while let [.., a, b] = blocks[..] {
            if a.1 / a.0 <= b.1 / b.0 {
                break;
            }
            blocks.pop();
            if let Some(last) = blocks.last_mut() {
                *last = (a.0 + b.0, a.1 + b.1, a.2 + b.2);
            }
        }
    }
    let means = blocks.iter().flat_map(|&(w, s, c)| std::iter::repeat_n(s / w, c));
    means.zip(offset).map(|(m, o)| m + o).collect()
}
