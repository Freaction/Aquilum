use super::bend::Bend;
use super::net::{Kind, Net};
use super::place::{Gaps, Ports, place};

const TRIES: usize = 8;
const STUCK: f64 = 4.0;
const STRETCH: f64 = 24.0;

fn stuck(net: &Net, layers: &[Vec<usize>], bend: &Bend) -> (f64, Option<(usize, usize, Option<usize>)>) {
    let (mut total, mut worst) = (0.0, (STUCK, None));
    for (r, layer) in layers.iter().enumerate() {
        for (k, &v) in layer.iter().enumerate() {
            let Some((target, _)) = bend.along(net, v) else { continue };
            let miss = target - net.items[v].x;
            total += miss.abs();
            if miss.abs() > worst.0 {
                let j = if miss > 0.0 { k + 1 } else { k.wrapping_sub(1) };
                let node = layer.get(j).filter(|&&u| net.items[u].kind == Kind::Node && net.items[u].owner == net.items[v].owner).map(|_| j);
                worst = (miss.abs(), Some((r, k, node)));
            }
        }
    }
    (total, worst.1)
}

pub(super) fn settle(net: &mut Net, layers: &mut [Vec<usize>], gaps: &Gaps, ports: Ports) {
    let mut bend = Bend::new(net);
    let mut extra = vec![0.0; layers.len()];
    place(net, layers, gaps, &extra, ports, &mut bend);
    for _ in 0..TRIES {
        let (before, Some((r, k, node))) = stuck(net, layers, &bend) else { break };
        let kept: Vec<(f64, f64, f64)> = net.items.iter().map(|i| (i.x, i.y, i.w)).collect();
        let mut fixed = false;
        for swap in [true, false] {
            match (swap, node) {
                (true, Some(j)) => layers[r].swap(k, j),
                (true, None) => continue,
                (false, _) => extra[r] += STRETCH,
            }
            place(net, layers, gaps, &extra, ports, &mut bend);
            if stuck(net, layers, &bend).0 < before - 1.0 {
                fixed = true;
                break;
            }
            match (swap, node) {
                (true, Some(j)) => layers[r].swap(k, j),
                _ => extra[r] -= STRETCH,
            }
            net.items.iter_mut().zip(&kept).for_each(|(i, &(x, y, w))| (i.x, i.y, i.w) = (x, y, w));
        }
        if !fixed {
            break;
        }
    }
}
