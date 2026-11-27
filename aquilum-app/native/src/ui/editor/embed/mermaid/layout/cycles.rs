pub(super) type Link = (usize, usize, i32, f64);

pub(super) fn adjacency(n: usize, links: &[Link]) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let (mut ins, mut outs) = (vec![Vec::new(); n], vec![Vec::new(); n]);
    for (k, l) in links.iter().enumerate() {
        outs[l.0].push(k);
        ins[l.1].push(k);
    }
    (ins, outs)
}

fn acyclic(n: usize, links: &mut [Link]) {
    let (_, outs) = adjacency(n, links);
    let mut state = vec![0u8; n];
    let mut back = Vec::new();
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        state[root] = 1;
        let mut stack = vec![(root, 0usize)];
        while let Some(&(v, i)) = stack.last() {
            let Some(&k) = outs[v].get(i) else {
                state[v] = 2;
                stack.pop();
                continue;
            };
            if let Some(top) = stack.last_mut() {
                top.1 += 1;
            }
            match state[links[k].1] {
                0 => {
                    state[links[k].1] = 1;
                    stack.push((links[k].1, 0));
                }
                1 => back.push(k),
                _ => {}
            }
        }
    }
    for k in back {
        let link = &mut links[k];
        std::mem::swap(&mut link.0, &mut link.1);
    }
}

fn greedy(n: usize, links: &mut [Link]) {
    let real: Vec<usize> = (0..links.len()).filter(|&k| links[k].3 > 0.0).collect();
    let (mut ins, mut outs) = (vec![0.0; n], vec![0.0; n]);
    for &k in &real {
        outs[links[k].0] += links[k].3;
        ins[links[k].1] += links[k].3;
    }
    let mut alive = vec![true; n];
    let (mut head, mut tail) = (Vec::new(), Vec::new());
    loop {
        let live = |v: &usize| alive[*v];
        let pick = if let Some(v) = (0..n).filter(live).find(|&v| outs[v] <= 0.0) {
            tail.push(v);
            v
        } else if let Some(v) = (0..n).filter(live).find(|&v| ins[v] <= 0.0) {
            head.push(v);
            v
        } else if let Some(v) = (0..n).filter(live).max_by(|&a, &b| (outs[a] - ins[a]).total_cmp(&(outs[b] - ins[b])).then(b.cmp(&a))) {
            head.push(v);
            v
        } else {
            break;
        };
        alive[pick] = false;
        for &k in &real {
            let (from, to, _, w) = links[k];
            if from == pick && alive[to] {
                ins[to] -= w;
            }
            if to == pick && alive[from] {
                outs[from] -= w;
            }
        }
    }
    let mut place = vec![0usize; n];
    head.iter().chain(tail.iter().rev()).enumerate().for_each(|(i, &v)| place[v] = i);
    for &k in &real {
        let link = &mut links[k];
        if place[link.0] > place[link.1] {
            std::mem::swap(&mut link.0, &mut link.1);
        }
    }
}

pub(super) fn untangle(n: usize, links: &mut [Link]) {
    greedy(n, links);
    acyclic(n, links);
}
