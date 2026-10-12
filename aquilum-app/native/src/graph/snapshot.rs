use std::path::Path;

use super::Graph;

const HEADER: usize = 16;

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

pub(super) fn decode(bytes: &[u8]) -> Option<(Vec<[f32; 2]>, Vec<f32>, Vec<f32>, Vec<u32>, Vec<[u32; 2]>, u64)> {
    let nodes = u32_at(bytes, 0)? as usize;
    let edges = u32_at(bytes, 4)? as usize;
    let epoch = u64::from(u32_at(bytes, 8)?) | (u64::from(u32_at(bytes, 12)?) << 32);
    let mut at = HEADER;
    let mut next_f32 = |count: usize| -> Option<Vec<f32>> {
        let values = (0..count).map(|i| f32_at(bytes, at + i * 4)).collect::<Option<Vec<_>>>()?;
        at += count * 4;
        Some(values)
    };
    let flat = next_f32(nodes * 2)?;
    let created = next_f32(nodes)?;
    let modified = next_f32(nodes)?;
    let degrees = (0..nodes).map(|i| u32_at(bytes, at + i * 4)).collect::<Option<Vec<_>>>()?;
    at += nodes * 4;
    let pairs = (0..edges).map(|i| Some([u32_at(bytes, at + i * 8)?, u32_at(bytes, at + i * 8 + 4)?])).collect::<Option<Vec<_>>>()?;
    let positions = flat.chunks_exact(2).map(|p| [p[0], p[1]]).collect();
    Some((positions, created, modified, degrees, pairs, epoch))
}

pub fn load(core: &aquilum_core::Core, workspace: &Path) -> Result<Graph, String> {
    let bytes = core.search.render_graph(&workspace.to_string_lossy()).map_err(|e| format!("{e}"))?;
    let (positions, created, modified, degrees, edges, epoch) = decode(&bytes).ok_or_else(|| "снимок графа повреждён".to_owned())?;
    let indices: Vec<u32> = (0..positions.len() as u32).collect();
    let paths = core.search.graph_paths(epoch, &indices).map_err(|e| format!("{e}"))?;
    Ok(Graph::new(positions, created, modified, degrees, edges, paths))
}
